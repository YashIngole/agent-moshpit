// Model-free stand-in that consumes the actual Claude/Codex launch configuration
// and uses the shipped stdio MCP subprocess. Used only by mcp.mjs.
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { appendFileSync } from 'node:fs'
import { createInterface } from 'node:readline'

const args = process.argv.slice(2)
let executable, serverArgs, serverEnv
if (args.includes('--mcp-config')) {
  const server = JSON.parse(args[args.indexOf('--mcp-config') + 1]).mcpServers.agent_moshpit
  executable = server.command
  serverArgs = server.args
  serverEnv = Object.fromEntries(Object.entries(server.env).map(([key, value]) => [key, value.replace(/\$\{([^}]+)\}/g, (_, name) => process.env[name])]))
} else {
  const settings = args.filter((_, i) => args[i - 1] === '-c')
  const setting = key => JSON.parse(settings.find(s => s.startsWith(`${key}=`)).slice(key.length + 1))
  executable = setting('mcp_servers.agent_moshpit.command')
  serverArgs = setting('mcp_servers.agent_moshpit.args')
  serverEnv = Object.fromEntries(setting('mcp_servers.agent_moshpit.env_vars').map(key => [key, process.env[key]]))
}
assert.ok(serverEnv.MOSHPIT_MCP_ENDPOINT)
assert.ok(serverEnv.MOSHPIT_MCP_TOKEN)
const bridge = spawn(executable, serverArgs, { env: { ...process.env, ...serverEnv }, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] })
let sequence = 0
const pending = new Map()
const output = createInterface({ input: bridge.stdout })
output.on('line', line => {
  const message = JSON.parse(line)
  const receiver = pending.get(message.id)
  pending.delete(message.id)
  if (receiver) receiver(message)
})
bridge.stderr.on('data', chunk => process.stderr.write(chunk))
const request = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence
  const timer = setTimeout(() => reject(new Error(`MCP timed out: ${method}`)), 35000)
  pending.set(id, message => { clearTimeout(timer); message.error ? reject(new Error(message.error.message)) : resolve(message.result) })
  bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
})
const call = async (name, args = {}, expectError = false) => {
  const result = await request('tools/call', { name, arguments: args })
  if (expectError) { assert.equal(result.isError, true); return result }
  if (result.isError) throw new Error(result.content[0].text)
  return result.structuredContent || JSON.parse(result.content[0].text)
}
const record = (event, value = {}) => appendFileSync(process.env.MOSHPIT_MCP_TEST_REPORT, JSON.stringify({ event, ...value }) + '\n')

try {
  const init = await request('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'moshpit-integration', version: '1' } })
  assert.equal(init.serverInfo.name, 'agent-moshpit')
  bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n')
  const definitions = await request('tools/list')
  assert.equal(definitions.tools.length, 11)
  const context = await call('get_context')
  const id = context.session.id
  const protectedName = context.title_locked
  await call('rename_session', { title: 'MCP test agent' }, protectedName)
  await call('set_activity', { activity: 'Checking Moshpit coordination' })
  record('connected', { id, cwd: context.session.cwd, protectedName })
  const complete = async task => {
    const full = await call('get_task', { task_id: task.id })
    await call('accept_task', { task_id: task.id })
    await call('set_activity', { activity: 'Reviewing delegated tests' })
    await call('report_result', { task_id: task.id, status: 'completed', summary: `Reviewed: ${full.task.prompt.split('\n')[0]}`, files: ['tests/example.test.ts'], validation: 'Model-free fixture passed' })
    process.stdout.write('\x1b]9;Agent turn complete\x07')
    record('reported', { id, taskId: task.id })
  }
  if (args.at(-1)?.startsWith('Parent MCP')) {
    await call('rename_session', { title: 'Coordinate API review' })
    const sessions = await call('list_sessions')
    assert.ok(sessions.sessions.some(row => row.session.id === id))
    // This parent is Codex: its delegated sessions are Codex too, with its permission boundary.
    const spec = { harness: 'codex', prompt: 'Review the API tests in this directory', title: 'API reviewer', request_key: 'initial-review' }
    const child = await call('start_session', spec)
    const repeat = await call('start_session', spec)
    assert.equal(repeat.reused, true)
    assert.equal(repeat.session_id, child.session_id)
    // Another program is refused, whatever its request key: its permissions could be broader.
    const other = await call('start_session', { ...spec, harness: 'claude', request_key: 'another-program' }, true)
    assert.match(other.content[0].text, /same program/)
    record('other-program-refused', { id })
    // A request key already used for another task is refused too.
    await call('start_session', { ...spec, prompt: 'Something else entirely' }, true)
    const result = await call('get_task', { task_id: child.task_id, wait_seconds: 25 })
    assert.equal(result.task.status, 'completed')
    assert.equal(result.task.validation, 'Model-free fixture passed')
    const followup = await call('send_task', { session_id: child.session_id, prompt: 'Review error handling next', request_key: 'followup' })
    assert.equal(followup.delivery, 'queued_for_next_inbox_check')
    const next = await call('get_task', { task_id: followup.task.id, wait_seconds: 25 })
    assert.equal(next.task.status, 'completed')
    record('delegation-passed', { id, childId: child.session_id, taskId: child.task_id, followupId: followup.task.id, cwd: context.session.cwd })
    process.stdout.write('\x1b]9;Agent turn complete\x07')
  } else {
    for (const task of context.inbox.assigned_tasks) await complete(task)
  }
  let reading = false
  setInterval(async () => {
    if (reading) return
    reading = true
    try {
      const inbox = await call('check_inbox')
      for (const task of inbox.assigned_tasks.filter(t => t.status === 'queued')) await complete(task)
    } catch (error) { record('error', { message: error.message }) }
    finally { reading = false }
  }, 300)
  console.log('MCP TEST AGENT READY')
  process.stdin.resume()
} catch (error) {
  record('error', { message: error.stack })
  bridge.kill()
  process.exitCode = 1
}
