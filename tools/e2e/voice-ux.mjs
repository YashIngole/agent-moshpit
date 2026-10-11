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
    await page.getByLabel('Enable local voice').check()
    await page.getByRole('button', { name: /Download selected model/ }).click()
    await page.getByRole('button', { name: 'Remove base model' }).waitFor()
    check(await page.getByRole('radio', { name: /Base ·/ }).isChecked(), `one setup download makes the selected model ready at ${width}`)
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
  await page.getByLabel('Enable local voice').check()
  await page.getByRole('button', { name: 'Download base model' }).click()
  await page.getByRole('button', { name: 'Remove base model' }).waitFor()
  await page.getByRole('button', { name: 'Close', exact: true }).click()
  await page.locator('[data-desk="demo-2"]').click()
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.getByRole('button', { name: 'Stop and insert' }).click()
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.locator('.pane[data-pane="demo-2"] .xterm-helper-textarea').evaluate(el => el === document.activeElement), 'Stop returns focus to the original terminal after insertion')
  check(await page.evaluate(() => window.__demo.typed.length === 1 && !/[\r\n]/.test(window.__demo.typed[0].data)), 'one recording inserts once and never submits Enter')
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
  check(await page.evaluate(n => window.__demo.typed.length === n + 1 && window.__demo.typed.at(-1).agent === 'demo-2', typed), 'letting go of a held shortcut finishes and types once')

  // A tap talks hands-free: it keeps listening until the next tap.
  await page.keyboard.press('Control+Shift+Space')
  await page.getByRole('button', { name: 'Stop and insert' }).waitFor()
  await new Promise(resolve => setTimeout(resolve, 700))
  check(await page.getByRole('button', { name: 'Stop and insert' }).count() === 1, 'a tap keeps listening hands-free')
  await page.keyboard.press('Control+Shift+Space')
  await page.getByText(/^Inserted\./).waitFor()
  check(await page.evaluate(n => window.__demo.typed.length === n + 2, typed), 'a second tap finishes')
  console.log(`all ${checks} voice UX checks passed`)
} finally {
  await browser?.close()
  server.kill()
}
