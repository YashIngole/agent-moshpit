// Screenshots of the mockups, for the plan in docs/.
//
//   node spikes/cli-office/mockups/shots.mjs
//
// Writes PNGs to docs/mockups/. Uses the Edge or Chrome already installed.
import { spawn } from 'node:child_process'
import { mkdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
const out = path.join(root, 'docs', 'mockups')
const PORT = 4174
const page_url = `http://localhost:${PORT}/spikes/cli-office/mockups/index.html`

const shots = [
  { name: '1-floor', screen: 'floor', size: [1040, 900], scheme: 'light' },
  { name: '2-terminal', screen: 'terminal', size: [1280, 800], scheme: 'light' },
  { name: '3-terminal-night', screen: 'terminal', size: [1280, 800], scheme: 'dark' },
  { name: '4-new-agent', screen: 'new', size: [1040, 800], scheme: 'light' },
  { name: '5-elsewhere', screen: 'elsewhere', size: [1040, 800], scheme: 'light' },
  { name: '6-side-by-side', screen: 'side', size: [1440, 900], scheme: 'light' },
  { name: '7-split-panes', screen: 'split', size: [1440, 900], scheme: 'light' },
  { name: '8-one-app', screen: 'desktop', size: [1440, 900], scheme: 'light' },
  { name: 'look-a-the-plan', screen: 'plan', size: [1440, 900], scheme: 'dark' },
  { name: 'look-b-the-wall', screen: 'wall', size: [1440, 900], scheme: 'dark' },
  { name: 'night-1-floor', screen: 'night-floor', size: [1440, 900], scheme: 'dark' },
  { name: 'night-2-terminals', screen: 'night-split', size: [1440, 900], scheme: 'dark' },
  { name: 'night-3-cast', screen: 'night-cast', size: [1440, 900], scheme: 'dark' }
]

const wanted = process.argv.slice(2)
async function reachable() {
  try {
    return (await fetch(page_url)).ok
  } catch {
    return false
  }
}

const vite = path.join(root, 'node_modules', 'vite', 'bin', 'vite.js')
let server
if (!(await reachable())) {
  server = spawn(process.execPath, [vite, '--port', String(PORT), '--strictPort'], { cwd: root, stdio: 'ignore' })
  for (let i = 0; i < 80 && !(await reachable()); i++) await new Promise(r => setTimeout(r, 250))
}

async function launch() {
  for (const channel of ['msedge', 'chrome']) {
    try {
      return await chromium.launch({ channel, headless: true })
    } catch {
      // try the next browser
    }
  }
  throw new Error('Neither Edge nor Chrome could be started.')
}

await mkdir(out, { recursive: true })
const browser = await launch()
try {
  for (const shot of shots.filter(s => wanted.length === 0 || wanted.includes(s.name))) {
    const [width, height] = shot.size
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 2, colorScheme: shot.scheme })
    const page = await context.newPage()
    const problems = []
    page.on('console', message => {
      if (message.type() === 'error') problems.push(message.text())
    })
    page.on('pageerror', error => problems.push(String(error)))
    await page.goto(`${page_url}?screen=${shot.screen}`)
    await page.waitForFunction(() => document.fonts.status === 'loaded')
    await page.waitForTimeout(500)
    await page.screenshot({ path: path.join(out, `${shot.name}.png`) })
    console.log(`${shot.name}.png  ${width}x${height}  ${shot.scheme}${problems.length ? `  ERRORS: ${problems.join(' | ')}` : ''}`)
    await context.close()
  }
} finally {
  await browser.close()
  server?.kill()
}
