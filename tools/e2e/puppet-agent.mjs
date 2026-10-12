// A model-free agent that does what a test tells it. It reads the launch configuration
// the office gives Codex or Claude Code, starts the shipped MCP subprocess from it, and
// then makes each call the test writes into its commands file. The answers, and anything
// the subprocess pushes without being asked, go into the report.
//
//   MOSHPIT_PUPPET_DIR   a folder for both: `<desk id>.commands.jsonl` is read, one
//                        `{"key":"k1","tool":"send_message","args":{...}}` a line, and
//                        `report.jsonl` is written.
//
// It can also run the hooks its launch was given, as Claude Code would at that moment:
// `{"key":"k2","hook":"PostToolBatch"}` runs that event's hook and reports what it printed;
// `{"key":"k3","hook":"wait"}` starts the one that waits, and reports when it ends.
//
// Used by the delivery tests. No model, account or credentials.
import { spawn } from 'node:child_process'
import { appendFileSync, existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'

const args = process.argv.slice(2)
const dir = process.env.MOSHPIT_PUPPET_DIR
const record = row => appendFileSync(path.join(dir, 'report.jsonl'), JSON.stringify(row) + '\n')
let executable, serverArgs, serverEnv
if (args.includes('--mcp-config')) {
  const server = JSON.parse(args[args.indexOf('--mcp-config') + 1]).mcpServers.agent_moshpit
  executable = server.command
  serverArgs = server.args
  serverEnv = Object.fromEntries(Object.entries(server.env).map(([key, value]) => [key, value.replace(/\$\{([^}]+)\}/g, (_, name) => process.env[name] ?? '')]))
} else {
  const settings = args.filter((_, i) => args[i - 1] === '-c')
  const setting = key => JSON.parse(settings.find(s => s.startsWith(`${key}=`)).slice(key.length + 1))
  executable = setting('mcp_servers.agent_moshpit.command')
  serverArgs = setting('mcp_servers.agent_moshpit.args')
  serverEnv = Object.fromEntries(setting('mcp_servers.agent_moshpit.env_vars').map(key => [key, process.env[key]]))
}
const bridge = spawn(executable, serverArgs, { env: { ...process.env, ...serverEnv }, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] })
let sequence = 0
let id = ''
const early = []
const pending = new Map()
createInterface({ input: bridge.stdout }).on('line', line => {
  let message
  // A line that is not one whole message would be two written together: the thing this guards against.
  try { message = JSON.parse(line) } catch { record({ event: 'garbled', id, line: line.slice(0, 200) }); return }
  if (message.id === undefined) {
    // One that comes before this session knows its own desk is kept until it does.
    const pushed = { event: 'pushed', method: message.method, params: message.params }
    if (id) record({ ...pushed, id }); else early.push(pushed)
    return
  }
  const receiver = pending.get(message.id)
  pending.delete(message.id)
  if (receiver) receiver(message)
})
bridge.stderr.on('data', chunk => process.stderr.write(chunk))
const request = (method, params = {}) => new Promise((resolve, reject) => {
  const n = ++sequence
  const timer = setTimeout(() => reject(new Error(`MCP timed out: ${method}`)), 35000)
  pending.set(n, message => { clearTimeout(timer); message.error ? reject(new Error(message.error.message)) : resolve(message.result) })
  bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: n, method, params }) + '\n')
})
const call = async (name, input = {}) => {
  const result = await request('tools/call', { name, arguments: input })
  if (result.isError) throw new Error(result.content[0].text)
  return result.structuredContent || JSON.parse(result.content[0].text)
}
/** The hooks this launch was given, by event, each as the program and words to start. */
const given = args.includes('--settings') ? JSON.parse(args[args.indexOf('--settings') + 1]).hooks ?? {} : {}
const hookFor = event => (event === 'wait' ? (given.Stop ?? []).flatMap(group => group.hooks).find(hook => hook.args?.includes('--mcp-wait')) : (given[event] ?? []).flatMap(group => group.hooks).find(hook => hook.args?.includes('--mcp-hook')))
/** Run one, with this process's own environment, as the CLI that was given it would. */
const runHook = (key, event) => new Promise(resolve => {
  const hook = hookFor(event)
  if (!hook) { resolve({ missing: true }); return }
  const child = spawn(hook.command, hook.args, { env: process.env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] })
  let stdout = ''
  let stderr = ''
  child.stdout.on('data', chunk => { stdout += chunk })
  child.stderr.on('data', chunk => { stderr += chunk })
  child.stdin.end(JSON.stringify({ hook_event_name: event, session_id: 'puppet' }))
  const started = Date.now()
  child.on('exit', code => {
    const ended = { code, stdout: stdout.trim(), stderr: stderr.trim(), ms: Date.now() - started }
    // The one that waits is reported when it ends; the others are the answer themselves.
    if (event === 'wait') record({ event: 'hook-ended', id, key, ...ended })
    else resolve(ended)
  })
  if (event === 'wait') resolve({ started: true })
})

try {
  // Asked for the newest revision, as Claude Code asks: what it is answered with is part of the report.
  const init = await request('initialize', { protocolVersion: '2026-07-28', capabilities: {}, clientInfo: { name: 'puppet', version: '1' } })
  bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n')
  const tools = (await request('tools/list')).tools.map(tool => tool.name)
  // Which desk this is, asked in a way that reads nothing: get_context lists the inbox, and
  // what is waiting for a session is the test's to read, when it means to.
  id = (await call('set_activity', { activity: '' })).session_id
  const own = (await call('list_sessions')).sessions.find(row => row.session.id === id)
  const commands = path.join(dir, `${id}.commands.jsonl`)
  // A program started again takes up where the test is now, not what its last run was told.
  let done = existsSync(commands) ? readFileSync(commands, 'utf8').split('\n').filter(Boolean).length : 0
  record({ event: 'connected', id, cwd: own.session.cwd, protocol: init.protocolVersion, capabilities: init.capabilities, tools, delivery: own.mcp, args })
  for (const pushed of early.splice(0)) record({ ...pushed, id })
  let busy = false
  setInterval(async () => {
    if (busy || !existsSync(commands)) return
    busy = true
    try {
      const lines = readFileSync(commands, 'utf8').split('\n').filter(Boolean)
      for (; done < lines.length; done++) {
        const { key, tool, args: input, hook } = JSON.parse(lines[done])
        try { record({ event: 'answer', id, key, result: hook ? await runHook(key, hook) : await call(tool, input) }) } catch (error) { record({ event: 'answer', id, key, error: error.message }) }
      }
    } finally { busy = false }
  }, 150)
  console.log('PUPPET AGENT READY')
  process.stdin.resume()
} catch (error) {
  record({ event: 'error', id, message: error.stack })
  bridge.kill()
  process.exitCode = 1
}
