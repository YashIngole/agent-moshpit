// Opt-in local model/WAV test in a named, isolated desktop office. NEVER opens
// a microphone: MOSHPIT_VOICE_WAV makes the debug build use prerecorded PCM.
// --download explicitly permits model downloads from the pinned public host.
import assert from 'node:assert/strict'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createHash } from 'node:crypto'
import { appPath, attach, closeWindow, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const fixtures = process.env.MOSHPIT_VOICE_FIXTURES
assert(fixtures, 'Set MOSHPIT_VOICE_FIXTURES to isolated fixture storage containing jfk.wav.')
assert(existsSync(path.join(fixtures, 'jfk.wav')), 'Provide whisper.cpp v1.8.3 samples/jfk.wav. No microphone is used.')
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-voice-e2e-'))
const instance = `voice${Date.now().toString(36)}`
const models = [
  { id: 'base', file: 'ggml-base-q5_1.bin', bytes: 59707625, sha256: '422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898' },
  { id: 'small', file: 'ggml-small-q5_1.bin', bytes: 190085487, sha256: 'ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb' }
]
mkdirSync(path.join(data, 'voice-models'))
for (const model of models) {
  const fixture = path.join(fixtures, model.file)
  if (existsSync(fixture)) copyFileSync(fixture, path.join(data, 'voice-models', model.file))
}
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([{ id: 'voice-fake', name: 'Voice Fake', tag: 'Fake', program: process.execPath, args: [path.join(here, 'fake-agent.mjs')] }]))
const { child, port } = launch({ port: 9247, env: { MOSHPIT_INSTANCE: instance, MOSHPIT_DATA_DIR: data, MOSHPIT_VOICE_WAV: path.join(fixtures, 'jfk.wav') } })
let browser
async function until(fn, timeout = 30_000) {
  const end = Date.now() + timeout
  while (Date.now() < end) { if (await fn()) return; await sleep(150) }
  throw new Error(`Voice check timed out after ${timeout}ms`)
}
try {
  const attached = await attach(port)
  browser = attached.browser
  const page = attached.page
  await page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke)
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  const view = () => invoke('voice_view')
  await until(async () => !(await view()).verifying)
  for (const model of models) {
    if (!(await view()).models.find(m => m.id === model.id).ready) {
      assert(process.argv.includes('--download'), `Missing ${model.file}. Use --download explicitly to test the product downloader.`)
      await invoke('voice_download', { model: model.id })
      await until(async () => !(await view()).downloading, 15 * 60_000)
      assert.equal((await view()).download_error, '')
    }
    const file = path.join(data, 'voice-models', model.file)
    assert.equal(statSync(file).size, model.bytes)
    assert.equal(createHash('sha256').update(readFileSync(file)).digest('hex'), model.sha256)
    assert(!existsSync(`${file}.part`), 'Only atomically completed models may remain')
    copyFileSync(file, path.join(fixtures, model.file))
    console.log(`ok verified ${model.id}: ${model.bytes} bytes`)
  }
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Voice Fake', { exact: true }).click()
  await page.getByRole('textbox', { name: 'Folder' }).fill(data)
  await page.getByLabel(/^Name/).fill('Voice fixture terminal')
  await page.getByRole('button', { name: 'Start agent' }).click()
  await page.locator('.pane').waitFor()
  const agent = await page.locator('.pane').getAttribute('data-pane')
  await until(async () => (await invoke('snapshot')).agents.find(a => a.id === agent)?.running)
  const settings = { enabled: true, model: 'base', language: 'english', shortcut: 'space' }
  for (const model of models) {
  await invoke('voice_config', { settings: { ...settings, model: model.id } })
  const before = await page.locator('.xterm-accessibility-tree').innerText()
  // Direct IPC allows deterministic recorded audio tests without any OS mic request.
  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  const started = Date.now()
  await invoke('voice_stop')
  await until(async () => !(await view()).busy, 180_000)
  const finished = await view()
  assert.equal(finished.phase, 'idle', finished.message)
  const text = await page.locator('.xterm-accessibility-tree').innerText()
  assert.match(text.replace(/\s+/g, ' '), /ask not/i)
  assert(text.length > before.length, 'This inference must insert new text')
  assert(!text.includes('YOU SAID:'), 'The fake program must not receive Enter')
  assert(!existsSync(path.join(data, 'screens', `${agent}.screen`)))
  console.log(`ok ${model.id} JFK WAV inserted without Enter; stop-to-result ${Date.now() - started}ms (one fixture, not a benchmark)`)
  }

  await invoke('voice_start', { agent })
  await until(async () => (await view()).phase === 'listening')
  closeWindow(child.pid)
  await sleep(500)
  const reopened = launch({ port, env: { MOSHPIT_INSTANCE: instance, MOSHPIT_DATA_DIR: data, MOSHPIT_VOICE_WAV: path.join(fixtures, 'jfk.wav') } })
  const next = await attach(port)
  await next.page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke)
  await browser.close()
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
  const kept = Object.keys(JSON.parse(readFileSync(path.join(data, 'settings.json'), 'utf8')))
  assert(kept.includes('voice'))
  console.log(`all voice fixture checks passed; isolated data: ${data}; executable: ${appPath()}`)
} finally {
  if (browser) await browser.close().catch(() => {})
  killTree(child.pid)
  forgetAddress(instance)
}
