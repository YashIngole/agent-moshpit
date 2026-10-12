// Opt-in desktop voice test in a named, isolated office. NEVER opens a microphone:
// MOSHPIT_VOICE_WAV makes the debug build hear a recorded file instead, as fast as it was
// spoken. The local engine needs its model in MOSHPIT_VOICE_FIXTURES (--download fetches it
// from the pinned public host); the cloud engine talks to a pretend Deepgram on this computer.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createHash } from 'node:crypto'
import { appPath, attach, closeWindow, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'
import { startMock } from './voice-mock.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const fixtures = process.env.MOSHPIT_VOICE_FIXTURES
assert(fixtures, 'Set MOSHPIT_VOICE_FIXTURES to isolated fixture storage containing jfk.wav.')
assert(existsSync(path.join(fixtures, 'jfk.wav')), 'Provide whisper.cpp v1.8.3 samples/jfk.wav. No microphone is used.')
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-voice-e2e-'))
const instance = `voice${Date.now().toString(36)}`
const model = { id: 'nemotron-streaming-en-q8', file: 'nemotron-speech-streaming-en-0.6b-Q8_0.gguf', bytes: 729650176, sha256: '90d8c89714cd31efc88be62a40c6b2bea57e0cc2063af1ffe2c28f1a228ca110' }
mkdirSync(path.join(data, 'voice-models'))
const fixture = path.join(fixtures, model.file)
if (existsSync(fixture)) copyFileSync(fixture, path.join(data, 'voice-models', model.file))
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([{ id: 'voice-fake', name: 'Voice Fake', tag: 'Fake', program: process.execPath, args: [path.join(here, 'fake-agent.mjs')] }]))
const mock = await startMock(9301)
const env = { MOSHPIT_INSTANCE: instance, MOSHPIT_DATA_DIR: data, MOSHPIT_VOICE_WAV: path.join(fixtures, 'jfk.wav'), MOSHPIT_VOICE_WAV_PACED: '1', MOSHPIT_VOICE_ENDPOINT: mock.url }
const { child, port } = launch({ port: 9247, env })
let browser
async function until(fn, timeout = 30_000) {
  const end = Date.now() + timeout
  while (Date.now() < end) { if (await fn()) return; await sleep(100) }
  throw new Error(`Voice check timed out after ${timeout}ms`)
}
/** The speech worker the app started, if one is running. */
function worker() {
  const out = execFileSync('powershell', ['-NoProfile', '-Command', `(Get-CimInstance Win32_Process -Filter "ParentProcessId=${child.pid}" | Where-Object { $_.CommandLine -like '*--voice-worker*' }).ProcessId`], { encoding: 'utf8' }).trim()
  return out ? Number(out.split(/\s+/)[0]) : null
}
const secret = 'mock-key-session-5678'
try {
  const attached = await attach(port)
  browser = attached.browser
  const page = attached.page
  await page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke)
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  const view = () => invoke('voice_view')
  // Without spaces: the terminal wraps long lines anywhere, even inside a word.
  const terminal = async () => (await page.locator('.xterm-accessibility-tree').innerText()).replace(/\s+/g, '')
  await until(async () => !(await view()).verifying)
  if (!(await view()).models.find(m => m.id === model.id).ready) {
    assert(process.argv.includes('--download'), `Missing ${model.file}. Use --download explicitly to test the product downloader.`)
    await invoke('voice_download', { model: model.id })
    await until(async () => !(await view()).downloading, 30 * 60_000)
    assert.equal((await view()).download_error, '')
  }
  const file = path.join(data, 'voice-models', model.file)
  assert.equal(statSync(file).size, model.bytes)
  assert.equal(createHash('sha256').update(readFileSync(file)).digest('hex'), model.sha256)
  assert(!existsSync(`${file}.part`), 'Only atomically completed models may remain')
  if (!existsSync(fixture)) copyFileSync(file, fixture)
  console.log(`ok ${model.id} is ready: ${model.bytes} bytes`)

  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Voice Fake', { exact: true }).click()
  await page.getByRole('textbox', { name: 'Folder' }).fill(data)
  // The name is folded into More options until asked for.
  if ((await page.locator('details.options').getAttribute('open')) === null) await page.locator('details.options > summary').click()
  await page.getByLabel(/^Name/).fill('Voice fixture terminal')
  await page.getByRole('button', { name: 'Start agent' }).click()
  await page.locator('.pane').waitFor()
  const agent = await page.locator('.pane').getAttribute('data-pane')
  await until(async () => (await invoke('snapshot')).agents.find(a => a.id === agent)?.running)
  const settings = { enabled: true, engine: 'local', local_model: model.id, shortcut: 'space', microphone: null, vocabulary: ['Moshpit'], send_desk_names: true }
  await invoke('voice_config', { settings })

  // ── on this computer: words are typed while the speaker is still talking ──
  const before = await terminal()
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  const listening = Date.now()
  await until(async () => /fellowamericans/i.test(await terminal()), 15_000)
  const first = Date.now() - listening
  assert.equal((await view()).phase, 'listening', 'The first words are typed before the recording stops')
  console.log(`ok local words typed while still listening: "fellow Americans" ${first} ms after listening began (said at about 3 s)`)
  await sleep(Math.max(0, 11_500 - (Date.now() - listening)))
  const stop = Date.now()
  await invoke('voice_stop')
  await until(async () => !(await view()).busy, 15_000)
  const settle = Date.now() - stop
  const finished = await view()
  assert.equal(finished.phase, 'idle', finished.message)
  await until(async () => /yourcountry/i.test(await terminal()), 5_000)
  const text = await terminal()
  assert.match(text, /fellowamericans.*asknotwhatyourcountrycandoforyou.*whatyoucandoforyourcountry/i)
  assert(text.length > before.length)
  assert(!text.includes('YOUSAID:'), 'The fake program must not receive Enter')
  assert(!existsSync(path.join(data, 'screens', `${agent}.screen`)))
  console.log(`ok local JFK typed in full without Enter; stop to idle ${settle} ms (debug build)`)

  // ── the speech worker dies mid-sentence: the recording says so, and the next one works ──
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  await until(() => worker() !== null, 10_000)
  await sleep(1500)
  execFileSync('taskkill', ['/F', '/PID', String(worker())], { stdio: 'ignore' })
  await until(async () => (await view()).phase === 'error', 10_000)
  assert.match((await view()).message, /stopped unexpectedly/)
  console.log('ok a killed speech worker ends the recording with a plain message')
  const typedBefore = (await terminal()).match(/fellowamericans/gi)?.length ?? 0
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  await until(async () => ((await terminal()).match(/fellowamericans/gi)?.length ?? 0) > typedBefore, 20_000)
  await invoke('voice_cancel')
  await until(async () => !(await view()).busy, 10_000)
  console.log('ok the next recording starts a fresh worker')

  // ── the cloud, with a key kept for this session, against a pretend Deepgram ──
  await invoke('voice_key_set', { provider: 'deepgram', key: secret, sessionOnly: true, check: false })
  const kept = (await view()).providers.find(p => p.id === 'deepgram')
  assert.deepEqual([kept.configured, kept.session_only, kept.last4], [true, true, '5678'])
  assert(!JSON.stringify(await view()).includes(secret), 'The key never comes back to the window')
  await invoke('voice_config', { settings: { ...settings, engine: 'deepgram' } })
  const phrases = async () => (await terminal()).match(/Mockphrase\d\./g)?.length ?? 0
  const sessions = mock.sessions.length
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  assert.equal((await view()).engine, 'deepgram')
  await until(async () => await phrases() >= 1, 10_000)
  assert.equal((await view()).phase, 'listening', 'Settled cloud words are typed before the recording stops')
  await sleep(2000)
  const cloudStop = Date.now()
  await invoke('voice_stop')
  await until(async () => !(await view()).busy, 10_000)
  const cloudSettle = Date.now() - cloudStop
  assert.equal((await view()).phase, 'idle', (await view()).message)
  await until(async () => (await terminal()).includes('Lastwords.'), 5_000)
  const session = mock.sessions[sessions]
  const query = new URL(session.url, 'ws://x').searchParams
  assert.equal(new URL(session.url, 'ws://x').pathname, '/v1/listen')
  assert.deepEqual([query.get('model'), query.get('language'), query.get('mip_opt_out'), query.get('encoding'), query.get('sample_rate')], ['nova-3', 'en', 'true', 'linear16', '16000'])
  for (const term of ['Moshpit', 'Voice fixture terminal', 'Voice Fake']) assert(query.getAll('keyterm').includes(term), `${term} is sent as a hint`)
  assert.equal(session.authorization, `Token ${secret}`)
  assert(session.messages.includes('Finalize') && session.messages.includes('CloseStream'), 'The stream is finished, then closed')
  await until(() => session.closed, 5_000)
  console.log(`ok cloud words typed while listening, last words after stop in ${cloudStop ? cloudSettle : 0} ms; the provider got English Nova-3, no training, the hints and the key in its header`)

  // Off: the desk's names are not sent.
  await invoke('voice_config', { settings: { ...settings, engine: 'deepgram', send_desk_names: false } })
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  await until(() => mock.sessions.length > sessions + 1, 5_000)
  await invoke('voice_cancel')
  await until(async () => !(await view()).busy, 10_000)
  const quiet = new URL(mock.sessions.at(-1).url, 'ws://x').searchParams.getAll('keyterm')
  assert(quiet.includes('Moshpit') && !quiet.includes('Voice fixture terminal'), 'Desk names stay home when turned off')
  await until(() => mock.sessions.at(-1).closed, 5_000)
  console.log('ok a cancelled cloud recording closes its connection, and desk names stay home when turned off')
  await invoke('voice_config', { settings: { ...settings, engine: 'deepgram' } })

  mock.mode = 'reject'
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'error', 10_000)
  assert.match((await view()).message, /Deepgram did not accept the key/)
  assert(!(await view()).message.includes(secret))
  console.log('ok a rejected key is explained without showing it')

  mock.mode = 'drop'
  const dropped = await phrases()
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'error', 15_000)
  assert.match((await view()).message, /Lost the connection to Deepgram/)
  assert(await phrases() > dropped, 'Words typed before the connection was lost stay')
  console.log('ok a lost connection keeps the words already typed')
  mock.mode = 'ok'

  // ── OpenRouter, against a pretend one: each phrase goes up whole at the pause after it ──
  await invoke('voice_key_set', { provider: 'openrouter', key: secret, sessionOnly: true, check: false })
  await invoke('voice_config', { settings: { ...settings, engine: 'openrouter' } })
  const routed = async () => (await terminal()).match(/Routerphrase\d+\./g)?.length ?? 0
  const uploads = mock.uploads.length
  const socketsBefore = mock.sessions.length
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  assert.equal((await view()).engine, 'openrouter')
  const routedFrom = Date.now()
  await until(async () => await routed() >= 1, 15_000)
  const firstPhrase = Date.now() - routedFrom
  assert.equal((await view()).phase, 'listening', 'A phrase is typed at its pause, before the recording stops')
  assert.equal((await view()).partial, '', 'There are no live words with a provider that is sent whole phrases')
  await sleep(Math.max(0, 11_500 - (Date.now() - routedFrom)))
  const routedStop = Date.now()
  await invoke('voice_stop')
  await until(async () => !(await view()).busy, 30_000)
  assert.equal((await view()).phase, 'idle', (await view()).message)
  const sent = mock.uploads.slice(uploads)
  assert(sent.length >= 2, `A phrase at a pause and the last one at the stop, got ${sent.length}`)
  await until(async () => await routed() === sent.length, 5_000)
  for (const upload of sent) {
    assert.deepEqual([upload.authorization, upload.model, upload.format, upload.language, upload.riff], [`Bearer ${secret}`, 'elevenlabs/scribe-v2', 'wav', 'en', 'RIFF'])
    // The audio and nothing else: no word hints, nothing about the desk.
    assert.deepEqual(upload.fields, ['input_audio', 'language', 'model'])
    assert(upload.samples >= 8000 && upload.samples <= 16 * 16000, `a phrase of ${upload.samples} samples`)
  }
  assert.equal(mock.sessions.length, socketsBefore, 'No socket is opened for OpenRouter')
  console.log(`ok OpenRouter: ${sent.length} phrases uploaded as WAV with the key and no hints; the first typed ${firstPhrase} ms after listening began, the last ${Date.now() - routedStop} ms after stop`)
  mock.mode = 'reject'
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'error', 30_000)
  assert.match((await view()).message, /OpenRouter did not accept the key/)
  assert(!(await view()).message.includes(secret))
  console.log('ok a key OpenRouter refuses is explained without showing it')
  mock.mode = 'ok'
  await invoke('voice_key_remove', { provider: 'openrouter' })

  // ── the system keychain: kept there, and gone when removed ──
  const stored = 'mock-key-keychain-9999'
  await invoke('voice_key_set', { provider: 'deepgram', key: stored, sessionOnly: false, check: false })
  const saved = (await view()).providers.find(p => p.id === 'deepgram')
  assert.deepEqual([saved.configured, saved.session_only, saved.last4], [true, false, '9999'])
  const listed = () => execFileSync('cmdkey', ['/list'], { encoding: 'utf8' })
  assert(listed().includes(`Agent Moshpit voice (${instance})`), 'The key is in Windows Credential Manager')
  await invoke('voice_key_remove', { provider: 'deepgram' })
  assert.equal((await view()).providers.find(p => p.id === 'deepgram').configured, false)
  assert(!listed().includes(`Agent Moshpit voice (${instance})`), 'A removed key leaves Credential Manager')
  console.log('ok a key is kept in the system keychain and removed from it')

  await invoke('voice_config', { settings })
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  const windowClosed = page.waitForEvent('close')
  closeWindow(child.pid)
  await windowClosed
  await browser.close()
  const reopened = launch({ port, env })
  const next = await attach(port)
  await next.page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke)
  browser = next.browser
  const afterClose = await next.page.evaluate(() => window.__TAURI_INTERNALS__.invoke('voice_view'))
  assert.equal(afterClose.phase, 'idle')
  assert.equal(afterClose.agent, null)
  console.log('ok window close into tray cancels fixture capture')
  if (reopened.child.pid !== child.pid) killTree(reopened.child.pid)
  await next.page.evaluate(agent => window.__TAURI_INTERNALS__.invoke('stop', { agent }), agent)
  await until(async () => !(await next.page.evaluate(() => window.__TAURI_INTERNALS__.invoke('snapshot'))).agents.find(a => a.id === agent)?.running)
  await next.page.evaluate(() => window.__TAURI_INTERNALS__.invoke('quit')).catch(() => {})
  await until(() => !isRunning(child.pid))
  assert(!existsSync(path.join(data, 'screens', `${agent}.screen`)), 'Voice echo must not be persisted on quit')
  // No key is written anywhere in the office's files.
  const files = dir => readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)])
  for (const name of files(data).filter(f => !f.endsWith('.gguf'))) {
    const body = readFileSync(name, 'latin1')
    assert(!body.includes(secret) && !body.includes(stored), `${name} holds no key`)
  }
  const keptSettings = JSON.parse(readFileSync(path.join(data, 'settings.json'), 'utf8'))
  assert.equal(keptSettings.voice.engine, 'local')
  assert(!('model' in keptSettings.voice) && !('language' in keptSettings.voice))
  console.log(`all voice fixture checks passed; isolated data: ${data}; executable: ${appPath()}`)
} finally {
  if (browser) await browser.close().catch(() => {})
  killTree(child.pid)
  forgetAddress(instance)
  await mock.close()
}
