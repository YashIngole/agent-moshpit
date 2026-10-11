// Focused voice recovery regressions. Simulated inputs; never opens a microphone.
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const port = 4186
const screenshots = path.join(root, '.impeccable/review')
mkdirSync(screenshots, { recursive: true })
const server = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), 'preview', '--port', String(port), '--strictPort'], { cwd: root, stdio: 'ignore', windowsHide: true })
let browser
let checks = 0
function check(value, message) { assert(value, message); checks++; console.log(`ok ${message}`) }
async function until(test, timeout = 3000) {
  const end = Date.now() + timeout
  while (Date.now() < end) {
    if (await test()) return true
    await new Promise(resolve => setTimeout(resolve, 50))
  }
  return test()
}
async function launchBrowser() {
  for (const channel of ['msedge', 'chrome']) {
    try { return await chromium.launch({ channel, headless: true }) } catch {}
  }
  return chromium.launch({ headless: true })
}
try {
  for (let i = 0; i < 40; i++) {
    try { if ((await fetch(`http://localhost:${port}`)).ok) break } catch {}
    await new Promise(resolve => setTimeout(resolve, 250))
  }
  browser = await launchBrowser()
  for (const width of [1280, 420]) {
    const page = await browser.newPage({ viewport: { width, height: 800 } })
    await page.goto(`http://localhost:${port}/?demo=office&still&voice=silence`)
    await page.getByRole('button', { name: 'Set up voice input' }).click()
    await page.getByLabel('Enable voice input').check()
    check(await page.getByRole('radio', { name: /^On this computer/ }).isChecked(), `voice starts on this computer at ${width}`)
    await page.getByRole('button', { name: /^Download Nemotron Speech Streaming · / }).click()
    await page.getByRole('button', { name: 'Remove Nemotron Speech Streaming', exact: true }).waitFor()
    check(await page.getByRole('radio', { name: /^Nemotron Speech Streaming ?730 MB.*recommended/ }).isChecked(), `one setup download makes the selected model ready at ${width}`)
    await page.getByLabel('Microphone', { exact: true }).selectOption('USB microphone')
    await page.getByRole('button', { name: 'Close', exact: true }).click()
    await page.locator('[data-desk="demo-2"]').click()
    await page.getByRole('button', { name: 'Start voice input' }).click()
    await page.getByRole('meter', { name: 'Microphone input level' }).waitFor()
    check(await page.getByRole('meter').getAttribute('aria-valuenow') === '0', `silent input is visible at ${width}`)
    await page.getByText(/No sound detected/).waitFor()
    check(await page.getByText(/No sound detected/).isVisible(), `silence gives actionable feedback during capture at ${width}`)
    await page.getByRole('button', { name: 'Stop and insert' }).click()
    await page.getByRole('button', { name: 'Try again', exact: true }).waitFor()
    check(await page.evaluate(() => window.__demo.typed.length === 0), `silence inserts nothing at ${width}`)
    await page.getByRole('button', { name: 'Try again', exact: true }).click()
    await page.getByRole('button', { name: 'Stop recording' }).waitFor()
    check(await page.getByRole('meter').count() === 1, `retry starts a single recording at ${width}`)
    await page.getByRole('button', { name: 'Cancel voice input' }).click()
    await page.getByRole('button', { name: 'Start voice input' }).waitFor()
    check(await page.getByRole('meter').count() === 0, `cancel clears recording feedback at ${width}`)
    await page.getByRole('button', { name: 'More', exact: true }).click()
    await page.getByRole('menuitem', { name: 'Voice input' }).click()
    check(await page.getByLabel('Microphone', { exact: true }).inputValue() === 'USB microphone', `selected microphone survives reopening setup at ${width}`)
    check(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `voice controls fit the viewport at ${width}`)
    await page.screenshot({ path: path.join(screenshots, `voice-panel-${width}-confirmed.png`) })
    await page.close()
  }
  const page = await browser.newPage()
  await page.goto(`http://localhost:${port}/?demo=office&still`)
  await page.getByRole('button', { name: 'Set up voice input' }).click()
  await page.getByLabel('Enable voice input').check()
  await page.getByRole('button', { name: 'Download Nemotron Speech Streaming', exact: true }).click()
  await page.getByRole('button', { name: 'Remove Nemotron Speech Streaming', exact: true }).waitFor()
  await page.getByRole('button', { name: 'Close', exact: true }).click()
  await page.locator('[data-desk="demo-2"]').click()
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.getByRole('button', { name: 'Stop and insert' }).click()
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.locator('.pane[data-pane="demo-2"] .xterm-helper-textarea').evaluate(el => el === document.activeElement), 'Stop returns focus to the original terminal after insertion')
  check(await page.evaluate(() => /^Fix the checkout total( and add a test)?$/.test(window.__demo.typed.map(t => t.data).join('')) && window.__demo.typed.every(t => !/[\r\n]/.test(t.data))), 'a recording types its words once, in order, and never submits Enter')
  await page.getByRole('button', { name: 'Dismiss voice notice' }).click()
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.getByRole('button', { name: 'Stop and insert' }).click()
  await page.locator('[data-desk="demo-6"]').click({ modifiers: ['ControlOrMeta'] })
  await page.locator('.pane[data-pane="demo-6"] .xterm-helper-textarea').focus()
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.locator('.pane[data-pane="demo-6"] .xterm-helper-textarea').evaluate(el => el === document.activeElement), 'a user who changes focus during transcription keeps their new terminal')
  check(await page.locator('.voice-status').count() === 1, 'the inserted notice is shown')
  await page.locator('.voice-status').waitFor({ state: 'detached', timeout: 4000 })
  check(true, 'the inserted notice goes away by itself')

  // Hold to talk: the shortcut held while speaking, and let go to finish.
  const typed = await page.evaluate(() => window.__demo.typed.length)
  await page.locator('[data-desk="demo-2"]').click()
  await page.locator('.pane[data-pane="demo-2"] .xterm-helper-textarea').focus()
  const box = async () => (await page.locator('.pane[data-pane="demo-6"]').boundingBox()).height
  const before = await box()
  await page.keyboard.down('Control')
  await page.keyboard.down('Shift')
  await page.keyboard.down('Space')
  await page.getByRole('button', { name: 'Stop and insert' }).waitFor()
  check(await box() === before, 'the voice card covers the terminals without resizing them')
  await page.locator('.voice-status .words', { hasText: 'Fix the checkout' }).waitFor()
  check(true, 'words appear while the shortcut is held')
  await page.keyboard.up('Space')
  await page.keyboard.up('Shift')
  await page.keyboard.up('Control')
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.evaluate(n => window.__demo.typed.slice(n).map(t => t.data).join('').startsWith('Fix the checkout total') && window.__demo.typed.slice(n).every(t => t.agent === 'demo-2'), typed), 'letting go of a held shortcut finishes and types the words once')

  // A tap talks hands-free: it keeps listening until the next tap.
  const tapped = await page.evaluate(() => window.__demo.typed.length)
  await page.getByText(/^Inserted\./).waitFor({ state: 'detached', timeout: 4000 })
  await page.keyboard.press('Control+Shift+Space')
  await page.getByRole('button', { name: 'Stop and insert' }).waitFor()
  await new Promise(resolve => setTimeout(resolve, 700))
  check(await page.getByRole('button', { name: 'Stop and insert' }).count() === 1, 'a tap keeps listening hands-free')
  await page.keyboard.press('Control+Shift+Space')
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.evaluate(n => window.__demo.typed.length > n && window.__demo.typed[n].data === 'Fix the checkout total', tapped), 'a second tap finishes')
  await page.close()

  // ── cloud engines with the user's own key ──
  async function cloudSetup(extra = '', width = 1280) {
    const p = await browser.newPage({ viewport: { width, height: 800 } })
    await p.goto(`http://localhost:${port}/?demo=office&still${extra}`)
    await p.getByRole('button', { name: 'Set up voice input' }).click()
    await p.getByLabel('Enable voice input').check()
    await p.getByRole('radio', { name: /^Cloud · your own key/ }).check()
    await p.getByLabel('Provider', { exact: true }).waitFor()
    return p
  }
  const secret = 'dg-secret-key-1234'
  let cloud = await cloudSetup()
  check(await cloud.getByLabel('Provider', { exact: true }).inputValue() === 'deepgram', 'cloud starts with Deepgram')
  check((await cloud.locator('.consent').innerText()).includes('goes to Deepgram'), 'choosing the cloud says where the audio goes')
  check((await cloud.locator('.readiness').innerText()).includes('Add your Deepgram key'), 'setup asks for the key before it is ready')
  check(await cloud.getByRole('button', { name: 'Get a Deepgram key' }).count() === 1 && await cloud.getByRole('button', { name: 'Deepgram pricing' }).count() === 1, 'the key and pricing pages are a click away')
  check(await cloud.getByLabel('Deepgram API key').getAttribute('type') === 'password', 'the key is typed into a hidden field')
  await cloud.getByLabel('Deepgram API key').fill(secret)
  await cloud.getByRole('button', { name: 'Save', exact: true }).click()
  await cloud.getByText(/Key added/).waitFor()
  check((await cloud.locator('.key').innerText()).includes('ends in 1234') && (await cloud.locator('.key').innerText()).includes('system keychain'), 'a saved key shows only its last four characters and where it is kept')
  check(await cloud.evaluate(s => !document.body.innerText.includes(s) && ![...document.querySelectorAll('input')].some(i => i.value.includes(s)), secret), 'the key is not left in the window')
  check(await cloud.evaluate(() => window.__demo.keys === 1), 'the key was kept once')
  await cloud.screenshot({ path: path.join(screenshots, 'voice-panel-cloud-1280.png'), fullPage: true })
  await cloud.getByRole('button', { name: 'Test', exact: true }).click()
  await cloud.getByText('Deepgram accepted the key.').waitFor()
  check(true, 'a kept key can be tested')
  await cloud.getByLabel('Add word hints').fill('Moshpit, fix/checkout-total, moshpit')
  await cloud.getByRole('button', { name: 'Add', exact: true }).click()
  await cloud.getByRole('button', { name: 'Remove Moshpit' }).waitFor()
  check(await cloud.locator('.chips li').count() === 2, 'word hints are added once each')
  await cloud.getByRole('button', { name: 'Remove Moshpit' }).click()
  check(await until(async () => await cloud.locator('.chips li').count() === 1), 'a word hint can be removed')
  check(await cloud.getByLabel("Also send the desk's name, folder and branch").isChecked(), 'desk names go as hints by default, and can be turned off')
  await cloud.getByLabel('Provider', { exact: true }).selectOption('openai')
  check(await until(async () => (await cloud.locator('.consent').innerText()).includes('goes to OpenAI')) && await cloud.getByLabel('OpenAI API key').count() === 1, 'each provider has its own key')
  await cloud.getByLabel('Provider', { exact: true }).selectOption('deepgram')
  await cloud.getByText(/Key added/).waitFor()
  await cloud.getByRole('button', { name: 'Close', exact: true }).click()
  await cloud.locator('[data-desk="demo-2"]').click()
  await cloud.getByRole('button', { name: 'Start voice input' }).click()
  check(await until(async () => (await cloud.locator('.voice-status').innerText()).includes('· Deepgram')), 'a cloud recording names its provider')
  await cloud.locator('.voice-status .words', { hasText: 'Fix the checkout' }).waitFor()
  await cloud.getByRole('button', { name: 'Stop and insert' }).click()
  await cloud.getByText(/^Inserted\./).waitFor()
  check(await cloud.evaluate(() => window.__demo.typed[0]?.data === 'Fix the checkout total'), 'a cloud recording types into the terminal')
  await cloud.getByRole('button', { name: 'More', exact: true }).click()
  await cloud.getByRole('menuitem', { name: 'Voice input' }).click()
  await cloud.getByRole('button', { name: 'Remove', exact: true }).click()
  check(await until(async () => (await cloud.locator('.readiness').innerText()).includes('Add your Deepgram key')), 'a removed key makes setup ask again')
  await cloud.close()

  cloud = await cloudSetup('&voice=store-unavailable', 420)
  await cloud.getByLabel('Deepgram API key').fill(secret)
  await cloud.getByRole('button', { name: 'Save', exact: true }).click()
  await cloud.getByRole('button', { name: 'Use the key for this session only' }).click()
  check(await until(async () => (await cloud.locator('.key').innerText()).includes('kept for this session only')), 'without a keychain the key can be kept for this session, never in a file')
  check(await cloud.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'cloud setup fits a narrow window')
  await cloud.screenshot({ path: path.join(screenshots, 'voice-panel-cloud-420.png'), fullPage: true })
  await cloud.close()

  cloud = await cloudSetup('&voice=offline')
  await cloud.getByLabel('Deepgram API key').fill(secret)
  await cloud.getByRole('button', { name: 'Save', exact: true }).click()
  await cloud.getByRole('button', { name: 'Save without checking' }).click()
  await cloud.getByText(/Key saved without checking/).waitFor()
  check(!(await cloud.locator('.key').innerText()).includes('unreached:'), 'an offline check offers to keep the key unchecked')
  await cloud.getByRole('button', { name: 'Close', exact: true }).click()
  await cloud.locator('[data-desk="demo-2"]').click()
  await cloud.getByRole('button', { name: 'Start voice input' }).click()
  check(await until(async () => ((await cloud.locator('.voice-status').textContent()) ?? '').includes('Deepgram could not be reached')) && await cloud.getByRole('button', { name: 'Voice setup' }).count() === 1, 'an unreachable provider is explained, with a way to setup')
  await cloud.close()

  cloud = await cloudSetup('&voice=auth')
  await cloud.getByLabel('Deepgram API key').fill(secret)
  await cloud.getByRole('button', { name: 'Save', exact: true }).click()
  check(await until(async () => (await cloud.getByRole('alert').innerText()).includes('did not accept the key')) && await cloud.getByRole('button', { name: /session only|without checking/ }).count() === 0, 'a rejected key is not kept')
  await cloud.close()

  cloud = await cloudSetup('&voice=quota')
  await cloud.getByLabel('Deepgram API key').fill(secret)
  await cloud.getByRole('button', { name: 'Save', exact: true }).click()
  await cloud.getByText(/Key added/).waitFor()
  await cloud.getByRole('button', { name: 'Close', exact: true }).click()
  await cloud.locator('[data-desk="demo-2"]').click()
  await cloud.getByRole('button', { name: 'Start voice input' }).click()
  check(await until(async () => ((await cloud.locator('.voice-status').textContent()) ?? '').includes('out of credit')) && await cloud.evaluate(() => window.__demo.typed.length === 0), 'an account out of credit is explained and types nothing')
  await cloud.close()

  const crash = await browser.newPage()
  await crash.goto(`http://localhost:${port}/?demo=office&still&voice=worker-crash`)
  await crash.getByRole('button', { name: 'Set up voice input' }).click()
  await crash.getByLabel('Enable voice input').check()
  await crash.getByRole('button', { name: 'Download Nemotron Speech Streaming', exact: true }).click()
  await crash.getByRole('button', { name: 'Remove Nemotron Speech Streaming', exact: true }).waitFor()
  await crash.getByRole('button', { name: 'Close', exact: true }).click()
  await crash.locator('[data-desk="demo-2"]').click()
  await crash.getByRole('button', { name: 'Start voice input' }).click()
  check(await until(async () => ((await crash.locator('.voice-status').textContent()) ?? '').includes('stopped unexpectedly'), 6000) && await crash.evaluate(() => window.__demo.typed.map(t => t.data).join('') === 'Fix the checkout total'), 'a crashed local engine says so and keeps the words already typed')
  await crash.close()

  const old = await browser.newPage()
  await old.goto(`http://localhost:${port}/?demo=office&still&voice=unsupported`)
  await old.getByRole('button', { name: 'Set up voice input' }).click()
  check(await old.getByLabel('Enable voice input').isEnabled(), 'an old processor can still turn voice on')
  await old.getByLabel('Enable voice input').check()
  check(await until(async () => (await old.locator('.readiness').innerText()).includes('Choose Cloud')), 'an old processor is pointed to the cloud')
  await old.close()
  console.log(`all ${checks} voice UX checks passed`)
} finally {
  await browser?.close()
  server.kill()
}
