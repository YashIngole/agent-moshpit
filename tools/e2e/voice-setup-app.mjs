// Native voice setup/IPC smoke test. Isolated office; never starts recording.
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { attach, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'

const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-voice-setup-'))
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
  assert.equal(initial.voice.enabled, false)
  assert.equal(initial.voice.model, 'base')
  assert.equal(initial.voice.microphone, null)
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
  await page.getByLabel('Enable local voice').check()
  await page.waitForFunction(() => !document.querySelector('input[type=checkbox]')?.disabled)
  const saved = JSON.parse(readFileSync(path.join(data, 'settings.json'), 'utf8'))
  assert.equal(saved.voice.microphone, selected || null)
  assert.equal(saved.voice.enabled, true)
  assert.equal(saved.voice.model, 'base')
  const view = await invoke('voice_view')
  assert.equal(view.phase, 'idle')
  assert.equal(view.busy, false)
  assert.equal(view.level, 0)
  assert.equal(view.agent, null)
  assert.equal(view.transcribing_ms, null)
  assert(!view.models.some(model => model.ready))
  await page.getByRole('button', { name: 'Close', exact: true }).click()
  await page.getByRole('button', { name: 'Set up voice input' }).click()
  assert.equal(await microphones.inputValue(), selected)
  assert.match(await page.locator('.readiness').innerText(), /Download Base/)
  console.log(`Native voice setup passed: ${inputs.devices.length} inputs listed, UI/IPC agree, microphone saved, Base default, setup opens no recording. Data: ${data}`)
  await invoke('quit').catch(() => {})
  for (let i = 0; i < 30 && isRunning(child.pid); i++) await sleep(150)
  assert(!isRunning(child.pid))
} finally {
  await browser?.close().catch(() => {})
  killTree(child.pid)
  forgetAddress(instance)
}
