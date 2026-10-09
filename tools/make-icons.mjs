// Renders the brand SVGs in assets/brand to the PNGs the app needs.
//
//   node tools/make-icons.mjs
//   npx tauri icon src-tauri/icons/app-icon.png
//
// The second command makes the .ico, .icns and sized PNGs from app-icon.png. It also
// writes phone and Store icons (android/, ios/, Square*, StoreLogo, 64x64) that this
// desktop app does not use; delete those afterwards.
//
// Uses the Chrome or Edge already on the machine, so nothing is downloaded.
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const brand = path.join(root, 'assets', 'brand')
const icons = path.join(root, 'src-tauri', 'icons')

const jobs = [
  { svg: 'icon.svg', out: 'app-icon.png', size: 1024 },
  { svg: 'tray.svg', out: 'tray.png', size: 64 },
  { svg: 'tray-attention.svg', out: 'tray-attention.png', size: 64 }
]

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

await mkdir(icons, { recursive: true })
const browser = await launch()
try {
  for (const job of jobs) {
    const svg = await readFile(path.join(brand, job.svg), 'utf8')
    const page = await browser.newPage({ viewport: { width: job.size, height: job.size }, deviceScaleFactor: 1 })
    await page.setContent(
      `<!doctype html><style>html,body{margin:0;background:transparent}svg{display:block;width:${job.size}px;height:${job.size}px}</style>${svg}`
    )
    const png = await page.screenshot({ omitBackground: true, type: 'png' })
    await writeFile(path.join(icons, job.out), png)
    await page.close()
    console.log(`${job.out}  ${job.size}x${job.size}  ${png.length} bytes`)
  }
} finally {
  await browser.close()
}
