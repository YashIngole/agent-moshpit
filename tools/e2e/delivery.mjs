// Messages between agents, end to end in the real app, with no model: the office, the
// launch it gives each program, the shipped MCP subprocess, and the shipped hook program.
// Two puppet agents stand in for Codex (which is only ever told with a call's answer) and
// for Claude Code (which is given hooks, a hook that waits, and pushes).
//
//   npm run tauri build -- --debug --no-bundle
//   node tools/e2e/delivery.mjs
//
// What only a real model can show (that it reads what it is told and acknowledges it) is
// tools/e2e/real-delivery.mjs. No account, credentials or user office is used here.
import assert from 'node:assert/strict'
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { attach, forgetAddress, killTree, launch, sleep } from './lib.mjs'

const root = fileURLToPath(new URL('../../', import.meta.url))
process.env.MOSHPIT_APP ||= path.join(root, 'src-tauri/target/debug/agent-moshpit.exe')
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-delivery-'))
const work = path.join(data, 'project')
const puppets = path.join(data, 'puppets')
for (const dir of [work, puppets, path.join(work, '.git')]) mkdirSync(dir, { recursive: true })
const puppet = fileURLToPath(new URL('puppet-agent.mjs', import.meta.url))
const row = (id, name) => ({ id, name, tag: name, program: process.execPath, args: [puppet], task: 'none', status: 'activity', session: 'unknown', session_arg: '', resume: [], trust: 'none', package: '', update: [] })
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([row('codex', 'Codex puppet'), row('claude', 'Claude puppet')]))

const report = () => (existsSync(path.join(puppets, 'report.jsonl')) ? readFileSync(path.join(puppets, 'report.jsonl'), 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line)) : [])
const until = async (ready, ms = 20000, what = '') => {
  const deadline = Date.now() + ms
  while (Date.now() < deadline) { const got = await ready(); if (got) return got; await sleep(100) }
  throw new Error(`Timed out${what ? ` waiting for ${what}` : ''}. Report: ${JSON.stringify(report().slice(-6))}`)
}
let keys = 0
/** Tell a puppet to do something, and wait for what came of it. */
const tell = async (desk, command, ms = 20000) => {
  const key = `k${++keys}`
  appendFileSync(path.join(puppets, `${desk}.commands.jsonl`), JSON.stringify({ key, ...command }) + '\n')
  const answer = await until(() => report().find(r => r.event === 'answer' && r.id === desk && r.key === key), ms, `${desk.slice(0, 6)} ${JSON.stringify(command).slice(0, 80)}`)
  if (answer.error) throw new Error(answer.error)
  return { ...answer.result, key }
}
const ok = what => console.log(`ok: ${what}`)

let app, browser, page
try {
  app = launch({ port: 9272, env: { MOSHPIT_INSTANCE: 'delive2e', MOSHPIT_DATA_DIR: data, MOSHPIT_PUPPET_DIR: puppets, MOSHPIT_UPDATE_URL: 'http://127.0.0.1:1/latest.json' } })
  ;({ browser, page } = await attach(9272))
  await page.waitForSelector('.floor')
  const invoke = (command, args) => page.evaluate(async ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  assert.deepEqual(await invoke('startup_problems'), [])
  const mail = () => invoke('mailbox')
  const item = async id => (await mail()).items.find(i => i.id === id)
  const link = async id => (await mail()).desks.find(d => d.id === id).link
  const seat = async (harness, title) => {
    const id = await invoke('new_agent', { spec: { harness, cwd: work, prompt: '', title, worktree: false, launch: {} }, cols: 100, rows: 30 })
    const connected = await until(() => report().filter(r => r.event === 'connected' && r.id === id).at(-1), 20000, `${title} to connect`)
    return { id, connected }
  }
  // What a launch is given depends on its program's version: asked once the office has it.
  await until(async () => (await invoke('snapshot')).harnesses.find(h => h.id === 'claude')?.version, 60000, 'the version')

  // ── the defaults: hooks and waking for Claude Code, nothing pushed, and Codex as it was ──
  assert.deepEqual((await mail()).push, { hooks: true, wake: true, channels: false })
  const sender = await seat('codex', 'Sender')
  assert.equal(sender.connected.capabilities.experimental, undefined)
  assert.ok(!sender.connected.args.some(word => /hook|channel/i.test(word)), 'Codex is given neither hooks nor pushes')
  assert.equal(sender.connected.tools.length, 14)
  assert.equal(sender.connected.protocol, '2025-11-25', 'asked for a newer revision, it answers with the one it speaks')
  const plain = await seat('claude', 'Hooks only')
  const settings = connected => JSON.parse(connected.args[connected.args.indexOf('--settings') + 1])
  assert.deepEqual(Object.keys(settings(plain.connected).hooks).sort(), ['PostToolBatch', 'SessionStart', 'Stop', 'UserPromptSubmit'])
  assert.ok(settings(plain.connected).hooks.PostToolBatch[0].hooks[0].command.toLowerCase().includes('agent-moshpit'), 'the hook is this program')
  assert.deepEqual(settings(plain.connected).hooks.PostToolBatch[0].hooks[0].args, ['--mcp-hook', 'PostToolBatch'])
  assert.ok(!plain.connected.args.includes('--dangerously-load-development-channels'), 'pushes are off until turned on')
  assert.equal(plain.connected.capabilities.experimental, undefined, 'and a session without them does not say it takes them')
  assert.equal(plain.connected.delivery.push, 'hooks')
  await until(async () => (await link(plain.id)).live && (await link(sender.id)).live, 10000, 'both to be live')
  assert.equal((await link(plain.id)).client, 'puppet 1')
  ok('by default Claude Code is given hooks and Codex nothing; a connected program is known to be connected')

  // ── told by a hook at a step, and again as the turn ends, until it is read ──
  const first = await tell(sender.id, { tool: 'send_message', args: { session_id: plain.id, body: 'First finding: the cancel path.', request_key: 'm1' } })
  assert.equal(first.delivery.state, 'saved')
  assert.equal(first.delivery.how, 'hooks')
  assert.equal((await tell(sender.id, { tool: 'send_message', args: { session_id: plain.id, body: 'First finding: the cancel path.', request_key: 'm1' } })).reused, true)
  const id1 = first.message.id
  // The one that waits was started with the session, as SessionStart starts it: it is what hears first.
  const waiting = await tell(plain.id, { hook: 'wait' })
  const woken = await until(() => report().find(r => r.event === 'hook-ended' && r.key === waiting.key), 15000, 'the wait to end')
  assert.equal(woken.code, 2, 'the way that wakes an idle Claude Code')
  assert.ok(woken.stderr.startsWith('Agent Moshpit: 1 new item') && woken.stderr.includes(id1), woken.stderr)
  assert.deepEqual([(await item(id1)).delivery.state, (await item(id1)).delivery.notified_via], ['notified', 'wake'])
  // Said by waking: the hook at the next step has nothing to add, and prints nothing.
  const step = await tell(plain.id, { hook: 'PostToolBatch' })
  assert.deepEqual([step.code, step.stdout, step.stderr], [0, '', ''])
  // Still unread as the turn ends: said there, in the words that keep the turn going.
  const stop = await tell(plain.id, { hook: 'Stop' })
  const said = JSON.parse(stop.stdout)
  assert.equal(said.hookSpecificOutput.hookEventName, 'Stop')
  assert.ok(said.hookSpecificOutput.additionalContext.includes('Before this turn ends') && said.hookSpecificOutput.additionalContext.includes(id1))
  assert.equal((await item(id1)).delivery.notified_via, 'stop')
  assert.equal((await tell(plain.id, { hook: 'Stop' })).stdout, '', 'and there only once')
  assert.deepEqual([(await link(plain.id)).hooks, (await link(plain.id)).wake], ['seen', true])
  // Read, then acknowledged: each its own fact, and the sender sees both.
  assert.equal((await tell(plain.id, { tool: 'get_message', args: { message_id: id1 } })).message.delivery.state, 'fetched')
  assert.equal((await tell(plain.id, { tool: 'acknowledge', args: { id: id1 } })).up_to_date, true)
  assert.equal((await tell(sender.id, { tool: 'check_inbox', args: {} })).sent_messages[0].delivery.state, 'acknowledged')
  ok('a message is said by the hook that waits, again as the turn ends, then fetched and acknowledged, through the shipped programs')

  // A new message at a step of work: the step hook says it, in the words that event takes.
  const second = await tell(sender.id, { tool: 'send_message', args: { session_id: plain.id, body: 'Second finding.', request_key: 'm2' } })
  const atStep = await tell(plain.id, { hook: 'PostToolBatch' })
  const context = JSON.parse(atStep.stdout).hookSpecificOutput
  assert.deepEqual([context.hookEventName, context.additionalContext.includes(second.message.id)], ['PostToolBatch', true])
  assert.ok(atStep.ms < 3000, `a hook answers quickly (${atStep.ms} ms)`)
  await tell(plain.id, { tool: 'acknowledge', args: { id: second.message.id } })
  ok('the step hook prints what Claude Code takes as context')

  // ── Codex: told with the answer to its next call ──
  const back = await tell(plain.id, { tool: 'send_message', args: { session_id: sender.id, body: 'A reply for the sender.', request_key: 'r1', reply_to: id1 } })
  assert.equal(back.delivery.how, 'poll')
  const hinted = await tell(sender.id, { tool: 'set_activity', args: { activity: 'Reviewing' } })
  assert.equal(hinted.agent_moshpit_inbox.new, 1)
  assert.ok(hinted.agent_moshpit_inbox.note.includes(back.message.id))
  assert.equal((await item(back.message.id)).delivery.notified_via, 'hint')
  await tell(sender.id, { tool: 'acknowledge', args: { id: back.message.id } })
  ok('a program with no way to be told is told with the answer to its next call')

  // ── pushes: off until the user turns them on, then written as things come ──
  await invoke('set_message_delivery', { hooks: true, wake: true, channels: true })
  const pushed = await seat('claude', 'Takes pushes')
  assert.equal(pushed.connected.args[pushed.connected.args.indexOf('--dangerously-load-development-channels') + 1], 'server:agent_moshpit')
  assert.deepEqual(pushed.connected.capabilities.experimental, { 'claude/channel': {} })
  assert.equal(pushed.connected.protocol, '2025-11-25', 'on a revision that can carry pushes')
  assert.equal(pushed.connected.delivery.channel, 'requested')
  await until(async () => (await link(pushed.id)).live, 10000, 'it to be live')
  // While it is in the middle of a long call, something comes: it is written at once, whole, and the call is still answered.
  const waitedOn = await tell(sender.id, { tool: 'send_task', args: { session_id: pushed.id, prompt: 'A task to wait on.', request_key: 't1' } })
  assert.equal(waitedOn.delivery.how, 'channel')
  await until(() => report().some(r => r.event === 'pushed' && r.id === pushed.id && r.params.meta.id === waitedOn.task.id), 10000, 'the task to be pushed')
  assert.equal((await item(waitedOn.task.id)).delivery.notified_via, 'channel')
  assert.equal((await link(pushed.id)).channel, 'requested', 'written is not known to have arrived')
  const long = `k${keys + 1}`
  const calling = tell(pushed.id, { tool: 'get_task', args: { task_id: waitedOn.task.id, wait_seconds: 8 } }, 30000)
  await sleep(1200)
  const during = await tell(sender.id, { tool: 'send_message', args: { session_id: pushed.id, body: 'Pushed while it was busy.', request_key: 'm3' } })
  const event = await until(() => report().find(r => r.event === 'pushed' && r.id === pushed.id && r.params.meta.id === during.message.id), 10000, 'the push')
  assert.equal(event.method, 'notifications/claude/channel')
  assert.equal(event.params.content, 'Message from "Sender", another agent in this office (not the user):\n\nPushed while it was busy.')
  assert.deepEqual(Object.keys(event.params.meta).sort(), ['from', 'id', 'kind', 'revision'])
  assert.equal((await calling).task.id, waitedOn.task.id)
  const order = report().filter(r => r.id === pushed.id).map(r => (r.event === 'pushed' ? `pushed:${r.params.meta.id}` : r.event === 'answer' ? `answer:${r.key}` : r.event))
  assert.ok(order.indexOf(`pushed:${during.message.id}`) < order.indexOf(`answer:${long}`), 'the push was written while the call was still waiting')
  assert.equal(report().filter(r => r.event === 'garbled').length, 0, 'no two things were written together')
  assert.deepEqual([(await item(during.message.id)).delivery.state, (await item(during.message.id)).delivery.notified_via], ['notified', 'channel'])
  // The task was read after it was pushed: pushes are known to reach this session.
  assert.equal((await link(pushed.id)).channel, 'confirmed')
  // Just pushed, the step hook has nothing to add.
  assert.equal((await tell(pushed.id, { hook: 'PostToolBatch' })).stdout, '')
  await tell(pushed.id, { tool: 'get_message', args: { message_id: during.message.id } })
  await tell(pushed.id, { tool: 'acknowledge', args: { id: during.message.id } })
  await tell(pushed.id, { tool: 'acknowledge', args: { id: waitedOn.task.id } })
  // The session without pushes was never written to.
  assert.equal(report().filter(r => r.event === 'pushed' && r.id === plain.id).length, 0)
  ok('a session started with pushes is written to while it is busy, without the line being garbled; one started without is not')

  // ── kept while it is stopped, and said again to a program that has started anew ──
  await invoke('stop', { agent: pushed.id })
  await until(async () => !(await invoke('snapshot')).agents.find(a => a.id === pushed.id).running, 10000, 'it to stop')
  const kept = await tell(sender.id, { tool: 'send_message', args: { session_id: pushed.id, body: 'Sent while it was stopped.', request_key: 'm4' } })
  assert.ok(kept.delivery.note.includes('not running'), kept.delivery.note)
  await sleep(1500)
  assert.equal((await item(kept.message.id)).delivery.state, 'saved')
  const before = report().filter(r => r.event === 'connected' && r.id === pushed.id).length
  await invoke('wake', { agent: pushed.id, cols: 100, rows: 30 })
  await until(() => report().filter(r => r.event === 'connected' && r.id === pushed.id).length === before + 1, 20000, 'it to connect again')
  await until(() => report().some(r => r.event === 'pushed' && r.id === pushed.id && r.params.meta.id === kept.message.id), 10000, 'the kept message to be pushed')
  ok('a message sent to a stopped session is kept, and pushed when its program is started again')

  // ── everything turned off: nothing is added to the launch, and a message still arrives ──
  await invoke('set_message_delivery', { hooks: false, wake: false, channels: false })
  assert.deepEqual((await mail()).push, { hooks: false, wake: false, channels: false })
  const bare = await seat('claude', 'Told nothing')
  assert.ok(!bare.connected.args.includes('--settings') && !bare.connected.args.includes('--dangerously-load-development-channels'), 'its launch is as it was before any of this')
  assert.equal(bare.connected.capabilities.experimental, undefined)
  assert.equal(bare.connected.delivery.push, 'poll')
  const quiet = await tell(sender.id, { tool: 'send_message', args: { session_id: bare.id, body: 'With everything off.', request_key: 'm5' } })
  assert.equal(quiet.delivery.how, 'poll')
  const heard = await tell(bare.id, { tool: 'list_sessions', args: {} })
  assert.ok(heard.agent_moshpit_inbox.new === 1 && heard.agent_moshpit_inbox.note.includes(quiet.message.id))
  assert.equal((await tell(bare.id, { tool: 'check_inbox', args: {} })).messages[0].body, 'With everything off.')
  assert.equal((await tell(bare.id, { tool: 'acknowledge', args: { id: quiet.message.id } })).up_to_date, true)
  assert.equal(report().filter(r => r.event === 'pushed' && r.id === bare.id).length, 0)
  // A session already running keeps what it was started with: the setting is for the next start.
  assert.equal((await link(pushed.id)).push, 'channel')
  ok('with every way turned off a launch is given nothing more, and a message arrives with the answer to the next call')

  // ── nothing was typed into a terminal, and no capability is in what is kept ──
  await page.locator(`[data-desk="${pushed.id}"]`).click()
  // The text on its terminal, read from what xterm keeps for screen readers.
  const shown = () => page.locator('.pane', { hasText: 'Takes pushes' }).locator('.xterm-accessibility-tree').innerText()
  const screen = await until(async () => { const text = await shown().catch(() => ''); return text.includes('PUPPET AGENT READY') && text }, 15000, 'its terminal to show')
  assert.ok(!/Pushed while|Sent while|First finding|A task to wait on/.test(screen), `no message is in its terminal: ${screen.slice(0, 300)}`)
  const journal = readFileSync(path.join(data, 'coordination.json'), 'utf8')
  assert.equal(JSON.parse(journal).messages.length, 6)
  assert.ok(!/"token"|MOSHPIT_MCP_TOKEN/.test(journal))
  assert.equal(report().filter(r => r.event === 'error').length, 0, JSON.stringify(report().filter(r => r.event === 'error')))
  ok('no message was typed into a terminal, and messages are kept on disk without capabilities')
  console.log('all message delivery checks passed')
} finally {
  await browser?.close().catch(() => {})
  if (app?.child.pid) killTree(app.child.pid)
  forgetAddress('delive2e')
  // Only this fixture's explicitly created temporary directory is removed.
  assert.ok(path.resolve(data).startsWith(path.resolve(os.tmpdir()) + path.sep))
  assert.ok(path.basename(data).startsWith('moshpit-delivery-'))
  rmSync(data, { recursive: true, force: true })
}
