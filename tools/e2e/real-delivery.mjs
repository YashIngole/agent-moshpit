// Message delivery to the real Claude Code, in a test instance of the office: the one
// thing delivery.mjs cannot show, that a model told of a message reads and acknowledges it.
// A model-free puppet desk sends; a real Claude Code desk receives, and nobody types.
//
// Not part of any suite. It starts YOUR Claude Code, signed in as you, on its smallest
// model, in an empty temporary folder, and answers its own start-up questions (the
// development-channels notice, and trusting that folder). Each run is a few short turns.
//
//   npm run tauri build -- --debug --no-bundle
//   node tools/e2e/real-delivery.mjs <how>-<when> [picture.png]
//
//   how    step      hooks at each step of its work, nothing that wakes it
//          wake      those, and the hook that waits (the default the app ships with)
//          channels  pushes alone (Claude Code's Channels preview)
//          both      everything
//          codex     the real Codex receives instead, which has none of these: it is told
//                    with the answer to its next call to the office (with `idle` only)
//   when   busy      sent while it is in the middle of a turn
//          idle      sent while it waits for a prompt
//          prompt    sent while it is stopped on a permission question
//          resume    sent while its program is stopped, then Carry on
//
// It ends with a RESULT line, and fails when the message was not acknowledged.
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { attach, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'

const [mode = 'wake-idle', picture] = process.argv.slice(2)
const [how, when] = mode.split('-')
if (!['step', 'wake', 'channels', 'both', 'codex'].includes(how) || !['busy', 'idle', 'prompt', 'resume'].includes(when) || (how === 'codex' && when !== 'idle')) {
  console.error('Say how and when, as in: node tools/e2e/real-delivery.mjs wake-idle')
  process.exit(2)
}
// Who receives. The stand-in that sends takes the other program's place in the table.
const real = how === 'codex' ? 'codex' : 'claude'
const name = real === 'codex' ? 'Real Codex' : 'Real Claude'
const root = fileURLToPath(new URL('../../', import.meta.url))
process.env.MOSHPIT_APP ||= path.join(root, 'src-tauri/target/debug/agent-moshpit.exe')
const scratch = mkdtempSync(path.join(os.tmpdir(), 'moshpit-real-delivery-'))
const data = path.join(scratch, 'data')
const project = path.join(scratch, 'project')
const puppets = path.join(scratch, 'puppets')
for (const dir of [data, project, puppets, path.join(project, '.git')]) mkdirSync(dir, { recursive: true })
for (const name of ['one', 'two', 'three', 'four', 'five']) writeFileSync(path.join(project, `${name}.txt`), `${name}\n`)
// Only the sender is a stand-in. The receiver is the program the office finds on this machine.
const stand = real === 'codex' ? 'claude' : 'codex'
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([{ id: stand, name: 'Puppet', tag: 'Puppet', program: process.execPath, args: [fileURLToPath(new URL('puppet-agent.mjs', import.meta.url))], task: 'none', status: 'activity', session: 'unknown', session_arg: '', resume: [], trust: 'none', package: '', update: [] }]))
const instance = 'realdeliv'
const began = Date.now()
const say = (...words) => console.log(`${((Date.now() - began) / 1000).toFixed(1).padStart(6)}s`, ...words)
const report = () => (existsSync(path.join(puppets, 'report.jsonl')) ? readFileSync(path.join(puppets, 'report.jsonl'), 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line)) : [])
const until = async (ready, ms, every = 400) => {
  const end = Date.now() + ms
  while (Date.now() < end) { const got = await ready(); if (got) return got; await sleep(every) }
  return null
}
const lines = (text, count) => text.split('\n').filter(line => line.trim()).slice(count).join('\n')
let child, browser
let passed = false
try {
  const started = launch({ port: 9271, env: { MOSHPIT_DATA_DIR: data, MOSHPIT_INSTANCE: instance, MOSHPIT_PUPPET_DIR: puppets, MOSHPIT_UPDATE_URL: 'http://127.0.0.1:1/latest.json' } })
  child = started.child
  const attached = await attach(started.port)
  browser = attached.browser
  const page = attached.page
  await page.waitForSelector('.floor')
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  const desk = async id => (await invoke('snapshot')).agents.find(a => a.id === id)
  const screen = () => page.locator('.pane', { hasText: name }).locator('.xterm-accessibility-tree').innerText().catch(() => '')
  const type = async (id, text) => { await invoke('term_write', { agent: id, data: text }); await sleep(400); await invoke('term_write', { agent: id, data: '\r' }) }
  const link = async id => (await invoke('mailbox')).desks.find(d => d.id === id)?.link
  const item = async id => (await invoke('mailbox')).items.find(i => i.id === id)

  const channels = how === 'channels' || how === 'both'
  const settings = { hooks: how !== 'channels', wake: how === 'wake' || how === 'both', channels }
  say('delivery settings:', JSON.stringify(settings))
  await invoke('set_message_delivery', settings)
  const version = await until(async () => (await invoke('snapshot')).harnesses.find(h => h.id === real)?.version, 60000)
  if (!version) throw new Error(`${real === 'codex' ? 'Codex' : 'Claude Code'} was not found on this machine.`)
  say(`${real === 'codex' ? 'Codex' : 'Claude Code'} version:`, version)

  const puppet = await invoke('new_agent', { spec: { harness: stand, cwd: project, prompt: '', title: 'Puppet sender', worktree: false, launch: {} }, cols: 100, rows: 30 })
  if (!await until(() => report().some(r => r.event === 'connected' && r.id === puppet), 20000)) throw new Error('The puppet did not connect.')

  // Codex is started so that it asks nothing: it may read, and is never to ask before a tool call.
  const launchAs = real === 'codex' ? { permission: 'custom', sandbox: 'read-only', approval: 'never' } : { model: 'haiku', ...(when === 'prompt' ? { permission: 'default' } : {}) }
  const claude = await invoke('new_agent', { spec: { harness: real, cwd: project, prompt: '', title: name, worktree: false, launch: launchAs }, cols: 110, rows: 34 })
  await page.locator(`[data-desk="${claude}"]`).click()
  // Its questions before it starts: its notice about development channels, and whether to trust the folder.
  let answered = 0
  const startup = async () => {
    const now = await desk(claude)
    const text = await screen()
    if (/local development/i.test(text) && /Enter to confirm|❯/.test(text) && answered < 6) {
      say('confirming its notice about development channels')
      await invoke('term_write', { agent: claude, data: '\r' })
      answered += 1
      await sleep(1500)
      return false
    }
    if (now.phase === 'needs_you' && /trust/i.test(now.activity + text) && answered < 6) {
      say('answering the trust question: Yes, I trust this folder')
      // "No, exit" is the one it starts on.
      await invoke('term_write', { agent: claude, data: '\x1b[B' })
      await sleep(300)
      await invoke('term_write', { agent: claude, data: '\r' })
      answered += 1
      await sleep(1500)
      return false
    }
    if (!now.running) throw new Error(`Claude Code ended (${now.phase}): ${now.activity}\n${lines(text, -30)}`)
    return now.phase === 'idle' ? now : false
  }
  // Codex is not answered for: a question from it here would be about your own settings.
  const codexReady = async () => {
    const now = await desk(claude)
    if (!now.running) throw new Error(`Codex ended (${now.phase}): ${now.activity}\n${lines(await screen(), -30)}`)
    return (await link(claude))?.live && !/trust/i.test(await screen()) ? now : false
  }
  if (!await until(real === 'codex' ? codexReady : startup, 90000, 700)) throw new Error(`It was not ready.\n${lines(await screen(), -30)}`)
  say('ready:', JSON.stringify(await link(claude)))

  const send = async (key, body) => {
    appendFileSync(path.join(puppets, `${puppet}.commands.jsonl`), JSON.stringify({ key, tool: 'send_message', args: { session_id: claude, body, request_key: key } }) + '\n')
    const answer = await until(() => report().find(r => r.event === 'answer' && r.key === key), 15000)
    if (!answer || answer.error) throw new Error(answer?.error ?? 'The puppet did not send.')
    say(`sent: ${answer.result.delivery.note}`)
    return answer.result.message.id
  }
  /** Say each change in how far it got, until it is acknowledged or time is up. */
  const follow = async (id, ms, label, each = async () => {}) => {
    let last = ''
    return until(async () => {
      await each()
      const now = await item(id)
      const state = `${now.delivery.state}${now.delivery.notified_via ? ` (via ${now.delivery.notified_via})` : ''}`
      if (state !== last) { say(`${label}: ${state}; desk ${(await desk(claude)).phase}`); last = state }
      return now.delivery.state === 'acknowledged'
    }, ms, 500)
  }
  // What it says asks for nothing: whether it is acknowledged is then the office's rule at work, not obedience to the sender.
  const body = 'Review note from the puppet desk: the cancel path in NewAgent.svelte looked fine on a second read. Nothing is needed from you.'

  if (when === 'resume') {
    // One turn first, so there is a conversation to carry on.
    await type(claude, 'Say hello in one word.')
    await until(async () => (await desk(claude)).phase === 'working', 20000, 200)
    await until(async () => ['idle', 'done'].includes((await desk(claude)).phase), 60000)
    await sleep(4000)
    await invoke('stop', { agent: claude })
    await until(async () => !(await desk(claude)).running, 20000)
    const id = await send('resume-1', body)
    await sleep(3000)
    say('while its program is stopped the message is', (await item(id)).delivery.state)
    await invoke('wake', { agent: claude, cols: 110, rows: 34 })
    let pressed = 0
    passed = Boolean(await follow(id, 150000, 'after Carry on', async () => {
      const text = await screen()
      if (/local development/i.test(text) && /Enter to confirm/.test(text) && pressed < 3) { await invoke('term_write', { agent: claude, data: '\r' }); pressed += 1; await sleep(1500) }
    }))
    say(passed ? 'RESULT: sent while it was stopped, told and acknowledged after Carry on, with nobody typing' : 'RESULT: NOT acknowledged within 150 s of Carry on')
  } else if (when === 'prompt') {
    // Stopped on a question only the user can answer: a message must not answer it, or type anything.
    await type(claude, 'Use the Write tool to create a file named note.txt in this folder containing the single word hi. Then reply DONE.')
    const asked = await until(async () => (await desk(claude)).phase === 'needs_you' && /note\.txt/.test(await screen()), 40000, 300)
    if (!asked) throw new Error(`Claude Code did not stop to ask (${(await desk(claude)).phase}).`)
    // Its question, once drawn whole: the same twice running.
    let before = ''
    await until(async () => { const text = lines(await screen(), -14); const same = text === before; before = text; return same }, 8000, 700)
    const id = await send('prompt-1', body)
    await sleep(25000)
    const after = lines(await screen(), -14)
    const now = await item(id)
    const untouched = after === before && !existsSync(path.join(project, 'note.txt')) && (await desk(claude)).phase === 'needs_you'
    say(`25 s later the question is ${after === before ? 'exactly as it was' : 'CHANGED'}, note.txt ${existsSync(path.join(project, 'note.txt')) ? 'EXISTS' : 'was not written'}, and the message is ${now.delivery.state}`)
    if (after !== before) say(`--- before ---\n${before}\n--- after ---\n${after}`)
    say('answering the question as the user would: No (Esc)')
    await invoke('term_write', { agent: claude, data: '\x1b' })
    passed = Boolean(await follow(id, 90000, 'after the question was answered')) && untouched
    say(passed ? 'RESULT: left alone at its question; told and acknowledged once the user had answered' : 'RESULT: its question was disturbed, or the message was not acknowledged within 90 s of the answer')
  } else if (when === 'busy') {
    await type(claude, 'Read the files one.txt, two.txt, three.txt, four.txt and five.txt in this folder, strictly one at a time: five separate Read tool calls, never in parallel, waiting for each result before the next call. After the fifth, reply with the single word DONE.')
    await until(async () => (await desk(claude)).phase === 'working', 30000, 200)
    await sleep(1500)
    const id = await send('busy-1', body)
    passed = Boolean(await follow(id, 120000, 'while busy'))
    say(passed ? 'RESULT: acknowledged while busy, with nobody typing' : 'RESULT: NOT acknowledged within 120 s')
  } else if (real === 'codex') {
    // Nothing reaches an idle Codex. With the answer to its next call to the office, it hears.
    const id = await send('codex-1', body)
    await sleep(15000)
    say('15 s later, idle, the message is', (await item(id)).delivery.state)
    // Said as a user would, with nothing about what else it may or may not do.
    await type(claude, 'Set your activity on your Agent Moshpit desk to: checking in')
    passed = Boolean(await follow(id, 150000, 'after a call to the office'))
    say(passed ? 'RESULT: Codex was told with the answer to its next call, and acknowledged' : `RESULT: Codex did NOT acknowledge within 150 s; the message is ${(await item(id)).delivery.state} (via ${(await item(id)).delivery.notified_via || 'nothing'})`)
  } else {
    const id = await send('idle-1', body)
    const woken = Boolean(await follow(id, 100000, 'while idle'))
    // With nothing that wakes it, an idle session hears with its next prompt: that is the design, not a failure.
    if (woken || settings.wake || channels) {
      passed = woken
      say(woken ? 'RESULT: an idle session was woken and acknowledged, with nobody typing' : 'RESULT: an idle session was NOT woken within 100 s')
    } else {
      say('not woken, as asked; typing a prompt, as a user returning would')
      await type(claude, 'Say hello in one word.')
      passed = Boolean(await follow(id, 90000, 'after a prompt'))
      say(passed ? 'RESULT: left idle; told with the next prompt, and acknowledged' : 'RESULT: NOT acknowledged after a prompt either')
    }
  }
  await until(async () => ['idle', 'done'].includes((await desk(claude)).phase), 60000)
  say('how it was reached, at the end:', JSON.stringify(await link(claude)))
  say(`--- its screen ---\n${lines(await screen(), -34)}`)
  if (picture) await page.screenshot({ path: picture })
  for (const id of [claude, puppet]) await invoke('dismiss', { agent: id })
  await sleep(800)
  await invoke('quit').catch(() => {})
  await browser.close().catch(() => {})
  await until(() => !isRunning(child.pid), 8000)
} finally {
  if (browser) await browser.close().catch(() => {})
  if (child) killTree(child.pid)
  forgetAddress(instance)
  await sleep(800)
  // Only this run's explicitly created temporary directory is removed.
  if (path.resolve(scratch).startsWith(path.resolve(os.tmpdir()) + path.sep) && path.basename(scratch).startsWith('moshpit-real-delivery-')) rmSync(scratch, { recursive: true, force: true })
}
process.exitCode = passed ? 0 : 1
