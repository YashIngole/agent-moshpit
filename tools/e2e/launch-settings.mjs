// Behavioral checks for launch settings, including a model released after load.
// The browser uses the demo bridge; no CLI task or model inference is performed.
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const port = 4176
const server = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), 'preview', '--port', String(port), '--strictPort'], { cwd: root, windowsHide: true, stdio: 'ignore' })
const base = `http://localhost:${port}/`
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))
let browser
const errors = []
try {
  for (let i = 0; i < 80; i++) {
    try { if ((await fetch(base)).ok) break } catch {}
    await sleep(100)
  }
  for (const channel of ['msedge', 'chrome']) {
    try { browser = await chromium.launch({ channel, headless: true }); break } catch {}
  }
  browser ??= await chromium.launch({ headless: true })
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } })
  page.on('pageerror', error => errors.push(error.message))
  await page.goto(`${base}?demo=empty&still&new-agent&catalog-new`)
  const model = page.getByLabel('Model', { exact: true })
  const effort = page.getByLabel('Effort', { exact: true })
  const permission = page.getByLabel('Permissions', { exact: true })
  await page.locator('#new-model option[value="opus"]').waitFor({ state: 'attached', timeout: 5000 }).catch(async error => {
    console.error(JSON.stringify({ errors, page: (await page.locator('body').innerText()).slice(-1600) }))
    throw error
  })
  assert.equal(await model.inputValue(), '')
  await model.selectOption('opus')
  await effort.selectOption('high')
  await permission.selectOption('auto')
  await page.getByRole('radiogroup').getByText('Codex', { exact: true }).click()
  await page.locator('#new-model option[value="codex-demo"]').waitFor({ state: 'attached' })
  assert.equal(await model.inputValue(), '', 'Claude settings must not leak into Codex')
  await model.selectOption('codex-demo')
  await effort.selectOption('xhigh')
  await permission.selectOption('yolo')
  await page.getByRole('radiogroup').getByText('Claude Code', { exact: true }).click()
  assert.equal(await model.inputValue(), 'opus')
  assert.equal(await effort.inputValue(), 'high')
  assert.equal(await permission.inputValue(), 'auto')
  console.log('ok   each provider keeps its own model, effort and permission draft')

  assert.equal(await page.locator('#new-model option[value="newly-released-model"]').count(), 0)
  await page.getByRole('button', { name: 'Refresh available models' }).click()
  await page.locator('#new-model option[value="newly-released-model"]').waitFor({ state: 'attached' })
  assert.equal(await model.inputValue(), 'opus', 'refresh must preserve the selected model')
  assert.equal(await effort.inputValue(), 'high')
  await model.selectOption('newly-released-model')
  await page.locator('#new-effort option[value="new-effort"]').waitFor({ state: 'attached' })
  await effort.selectOption('new-effort')
  console.log('ok   a new model and a new effort level appear on refresh without an app update')

  await page.getByRole('button', { name: 'Cancel', exact: true }).click()
  await page.getByRole('button', { name: 'New agent' }).first().click()
  assert.equal(await model.inputValue(), 'newly-released-model')
  assert.equal(await effort.inputValue(), 'new-effort')
  await page.getByLabel('Folder', { exact: true }).fill('C:/code/catalog-project')
  await page.getByLabel(/What should they do/).fill('Check the launch settings')
  await page.getByRole('button', { name: 'Start agent', exact: true }).click()
  const fresh = page.locator('[data-desk^="demo-new-"]').first()
  await fresh.waitFor()
  await fresh.click({ button: 'right' })
  await page.getByRole('menuitem', { name: /Start another like this/ }).click()
  assert.equal(await model.inputValue(), 'newly-released-model')
  assert.equal(await effort.inputValue(), 'new-effort')
  assert.equal(await permission.inputValue(), 'auto')
  console.log('ok   close/reopen and duplication retain explicit launch settings')

  await page.getByText('Advanced launch settings', { exact: true }).click()
  await page.getByLabel('Allow without prompting', { exact: false }).fill('Read\nBash(git diff *)')
  await page.getByLabel('Extra instructions', { exact: false }).fill('Run the relevant checks.')
  await page.getByRole('button', { name: 'Reset to CLI settings' }).click()
  assert.equal(await model.inputValue(), '')
  assert.equal(await permission.inputValue(), '')
  await page.getByText('Advanced launch settings', { exact: true }).click()

  // One visual inspection batch: both providers and the narrow desktop window.
  const review = path.join(root, '.impeccable/review')
  mkdirSync(review, { recursive: true })
  await page.addStyleTag({ content: '*,*::before,*::after { animation: none !important; transition: none !important; }' })
  await page.locator('aside.panel form').evaluate(form => { form.scrollTop = 0 })
  await page.screenshot({ path: path.join(review, 'desktop.png'), fullPage: true })
  await page.getByRole('radiogroup').getByText('Codex', { exact: true }).click()
  await page.locator('#new-model option[value="codex-demo"]').waitFor({ state: 'attached' })
  await page.getByText('Models come from your CLI. Refresh to check for new releases.', { exact: true }).waitFor()
  await page.locator('aside.panel form').evaluate(form => { form.scrollTop = 0 })
  await page.screenshot({ path: path.join(review, 'desktop-codex.png'), fullPage: true })
  await permission.selectOption('custom')
  await page.getByLabel('Sandbox', { exact: true }).selectOption('workspace-write')
  await page.getByLabel('Approval policy', { exact: true }).selectOption('never')
  assert.equal(await page.locator('details').getAttribute('open'), '')
  await page.setViewportSize({ width: 390, height: 760 })
  await permission.selectOption('workspace')
  await page.getByText('Advanced launch settings', { exact: true }).click()
  await page.locator('aside.panel form').evaluate(form => { form.scrollTop = 0 })
  await page.screenshot({ path: path.join(review, 'mobile.png'), fullPage: true })
  assert.equal(await page.locator('aside.panel').evaluate(panel => panel.scrollWidth <= panel.clientWidth), true)
  assert.equal(await page.getByRole('button', { name: 'Start agent', exact: true }).isVisible(), true)
  console.log('ok   advanced controls, reset, custom sandbox and narrow-window layout')

  await page.goto(`${base}?demo=empty&still&new-agent&catalog-fails`)
  await page.getByText(/The model catalog could not be read/).waitFor()
  await model.selectOption('__custom__')
  await page.getByLabel('Model ID', { exact: true }).fill('a-model-released-today')
  await page.getByLabel('Folder', { exact: true }).fill('C:/code/offline')
  await page.getByRole('button', { name: 'Start agent', exact: true }).click()
  await page.locator('[data-desk^="demo-new-"]').first().waitFor({ state: 'attached' })
  assert.deepEqual(errors, [], 'there must be no runtime errors')
  console.log('ok   failed discovery still permits inherited settings and custom model IDs')

  await page.setViewportSize({ width: 1280, height: 800 })
  for (const [label, value] of [
    ['Additional folders', 'C:/code/shared\nC:/code/utilities'],
    ['Allow without prompting', 'Read\nBash(git diff *)'],
    ['Deny tools', 'Bash(rm *)\nWebFetch'],
  ]) {
    await page.goto(`${base}?demo=empty&still&new-agent`)
    await page.getByLabel('Folder', { exact: true }).fill('C:/code/keyboard-launch')
    await page.getByText('Advanced launch settings', { exact: true }).click()
    const field = page.getByLabel(label, { exact: false })
    await field.fill(value)
    await field.press('Control+Enter')
    const desk = page.locator('[data-desk^="demo-new-"]').first()
    await desk.waitFor()
    await desk.click({ button: 'right' })
    await page.getByRole('menuitem', { name: /Start another like this/ }).click()
    await page.getByText('Advanced launch settings', { exact: true }).click()
    assert.equal(await field.inputValue(), value, `${label} must be committed when Ctrl+Enter starts the agent`)
  }
  assert.deepEqual(errors, [], 'keyboard launches must have no runtime errors')
  console.log('ok   Ctrl+Enter commits multiline folders and tool rules before starting')
} finally {
  if (browser) await browser.close()
  server.kill()
}
