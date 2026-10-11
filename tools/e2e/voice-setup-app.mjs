// Native voice setup/IPC smoke test. Isolated office; never starts recording.
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { attach, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'

const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-voice-setup-'))
// Settings as 0.6 wrote them, with one value from a version not yet made: each setting is read
// on its own, so the unknown one takes its default and the rest are kept.
writeFileSync(path.join(data, 'settings.json'), JSON.stringify({ close_quits: true, voice: { enabled: false, model: 'small', language: 'hindi', shortcut: 'altspace', microphone: null, engine: 'from-the-future' } }))
const instance = `voicesetup${Date.now().toString(36)}`
const { child, port } = launch({ port: 9248, env: { MOSHPIT_INSTANCE: instance, MOSHPIT_DATA_DIR: data } })
let browser
try {
  const connected = await attach(port)
  browser = connected.browser
  const page = connected.page
  await page.waitForFunction(() => window.__TAURI_INTERNALS__?.invoke)
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  const initial = await invoke('settings')
  assert.equal(initial.close_quits, true)
  assert.equal(initial.voice.enabled, false)
  assert.equal(initial.voice.engine, 'local')
  assert.equal(initial.voice.local_model, 'nemotron-streaming-en-q8')
  assert.equal(initial.voice.shortcut, 'altspace', 'A setting from 0.6 is kept')
  assert.equal(initial.voice.microphone, null)
  assert.equal(initial.voice.send_desk_names, true)
  const inputs = await invoke('voice_inputs')
  assert(Array.isArray(inputs.devices))
  assert(inputs.devices.every(name => typeof name === 'string'))
  await page.getByRole('button', { name: 'Set up voice input' }).click()
  const microphones = page.getByLabel('Microphone', { exact: true })
  await microphones.waitFor()
  await page.getByRole('button', { name: 'Refresh microphones' }).click()
  await page.getByRole('button', { name: 'Refresh microphones' }).waitFor({ state: 'visible' })
  await page.waitForFunction(() => !document.querySelector('#voice-microphone')?.disabled)
  const options = await microphones.locator('option').evaluateAll(list => list.map(option => option.value))
  assert.deepEqual(options, ['', ...inputs.devices])
  const selected = inputs.devices[0] ?? ''
  await microphones.selectOption(selected)
  await page.waitForFunction(() => !document.querySelector('input[type=checkbox]')?.disabled)
  await page.getByLabel('Enable voice input').check()
  await page.waitForFunction(() => !document.querySelector('input[type=checkbox]')?.disabled)
  const saved = () => JSON.parse(readFileSync(path.join(data, 'settings.json'), 'utf8')).voice
  assert.equal(saved().microphone, selected || null)
  assert.equal(saved().enabled, true)
  assert.equal(saved().local_model, 'nemotron-streaming-en-q8')
  assert(!('model' in saved()) && !('language' in saved()), 'Settings 0.7 no longer uses are not written back')
  const view = await invoke('voice_view')
  assert.equal(view.phase, 'idle')
  assert.equal(view.busy, false)
  assert.equal(view.level, 0)
  assert.equal(view.agent, null)
  assert.equal(view.transcribing_ms, null)
  assert.equal(view.unsupported, '', 'This computer can run the local engine')
  assert(!view.models.some(model => model.ready))
  assert.deepEqual(view.models.map(m => m.id), ['nemotron-streaming-en-q8', 'nemotron-streaming-en-q4'])
  assert.deepEqual(view.providers.map(p => [p.id, p.configured]), [['deepgram', false], ['openai', false], ['assemblyai', false], ['elevenlabs', false]])
  await page.getByRole('button', { name: 'Close', exact: true }).click()
  await page.getByRole('button', { name: 'Set up voice input' }).click()
  assert.equal(await microphones.inputValue(), selected)
  assert.match(await page.locator('.readiness').innerText(), /Download Nemotron Speech Streaming/)
  await page.getByRole('radio', { name: /^Cloud · your own key/ }).check()
  await page.getByLabel('Deepgram API key').waitFor()
  await page.waitForFunction(() => !document.querySelector('#voice-provider-choice')?.disabled)
  assert.equal(saved().engine, 'deepgram')
  assert.match(await page.locator('.readiness').innerText(), /Add your Deepgram key/)
  await page.getByRole('radio', { name: /^On this computer/ }).check()
  await page.getByRole('heading', { name: 'Local model' }).waitFor()
  await page.waitForFunction(() => !document.querySelector('input[name=voice-engine]')?.disabled)
  assert.equal(saved().engine, 'local')
  console.log(`Native voice setup passed: 0.6 settings carried over, ${inputs.devices.length} inputs listed, UI/IPC agree, microphone saved, engine choice saved, setup opens no recording. Data: ${data}`)
  await invoke('quit').catch(() => {})
  for (let i = 0; i < 30 && isRunning(child.pid); i++) await sleep(150)
  assert(!isRunning(child.pid))
} finally {
  await browser?.close().catch(() => {})
  killTree(child.pid)
  forgetAddress(instance)
}
