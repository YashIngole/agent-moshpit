// The pictures on agentmoshpit.com, taken from the app's own window with its demo data.
//
//   npm run build:site && node tools/site-shots.mjs
//
// Writes into site/img:
//   terminals.webp, terminals-narrow.webp   the floor with terminals beside it (the page's opening)
//   office.webp                             the floor by itself
//   pose-<status>.webp                      one person for each status, as the app draws them
//   card.png                                what a link to the site shows
// The window is drawn at twice its size, so the pictures stay sharp on a dense screen.
// Run it again when the window's look changes. Uses the Edge or Chrome already installed.
//
//   node tools/site-shots.mjs --serve     only serve site/ on http://localhost:4175, to look at it
import { createReadStream, existsSync, statSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const site = path.join(root, 'site')
const PORT = 4175
const base = `http://localhost:${PORT}/`

const TYPES = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.png': 'image/png', '.webp': 'image/webp', '.woff2': 'font/woff2' }

/** site/ as a web server would hand it out: nothing clever, and nothing outside the folder. */
const server = createServer((request, response) => {
  const asked = decodeURIComponent(new URL(request.url, base).pathname)
  let file = path.join(site, asked)
  if (!file.startsWith(site)) return response.writeHead(403).end()
  if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html')
  if (!existsSync(file)) return response.writeHead(404).end('not found')
  response.writeHead(200, { 'content-type': TYPES[path.extname(file)] ?? 'application/octet-stream' })
  createReadStream(file).pipe(response)
})
// Already being served (by `--serve` in another terminal): that one will do.
const serving = await new Promise(resolve => server.once('error', () => resolve(false)).listen(PORT, () => resolve(true)))

if (process.argv.includes('--serve')) {
  console.log(serving ? `site/ is at ${base}  (Ctrl+C to stop)` : `Something is already serving ${base}`)
} else {
  if (!existsSync(path.join(site, 'demo', 'index.html'))) {
    console.error('site/demo is not there: run `npm run build:site` first.')
    process.exit(1)
  }
  const out = path.join(site, 'img')
  await mkdir(out, { recursive: true })

  async function launch() {
    for (const channel of ['msedge', 'chrome']) {
      try {
        return await chromium.launch({ channel, headless: true })
      } catch {
        // try the next browser
      }
    }
    return chromium.launch({ headless: true })
  }

  /** A PNG as WebP, made by the browser itself: a fraction of the size for a picture of a dark window. */
  async function webp(page, png, quality = 0.92) {
    const data = await page.evaluate(
      async ([source, q]) => {
        const image = await createImageBitmap(await (await fetch(source)).blob())
        const canvas = new OffscreenCanvas(image.width, image.height)
        canvas.getContext('2d').drawImage(image, 0, 0)
        const blob = await canvas.convertToBlob({ type: 'image/webp', quality: q })
        return Array.from(new Uint8Array(await blob.arrayBuffer()))
      },
      [`data:image/png;base64,${png.toString('base64')}`, quality]
    )
    return Buffer.from(data)
  }

  const browser = await launch()
  try {
    /** The demo office in a window of this size, drawn at twice the size. */
    const office = async (width, height) => {
      const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 2, colorScheme: 'dark' })
      // A pretend dense screen draws a terminal at the wrong size on the graphics card (a real
      // one does not), so the terminals here are drawn the plain way: the same letters, as text.
      await context.route(/addon-webgl/, route => route.abort())
      const page = await context.newPage()
      await page.goto(`${base}demo/?demo=office&still`)
      await page.waitForSelector('.floor button[data-desk]')
      await page.waitForFunction(() => document.fonts.status === 'loaded')
      return page
    }
    const desk = (page, n) => page.locator(`.floor button[data-desk="demo-${n}"]`)
    const save = async (page, name, said) => {
      await writeFile(path.join(out, name), await webp(page, await page.screenshot()))
      console.log(`${name.padEnd(24)} ${said}`)
    }

    // Two agents at work and one idle, side by side with the floor: the page's opening.
    let page = await office(1240, 800)
    await desk(page, 2).click()
    await desk(page, 5).click({ modifiers: ['ControlOrMeta'] })
    await desk(page, 6).click({ modifiers: ['ControlOrMeta'] })
    await page.waitForFunction(() => document.querySelectorAll('.pane').length === 3)
    await page.waitForTimeout(600)
    // The floor follows the desk opened last; the picture starts at its first room.
    await page.evaluate(() => document.querySelector('.floor').scrollTo(0, 0))
    await page.waitForTimeout(600)
    await save(page, 'terminals.webp', '1240x800 at 2x')
    await page.context().close()

    // In a window kept narrow the terminals take it all: one is shown.
    page = await office(420, 560)
    await desk(page, 2).click()
    await page.waitForFunction(() => document.querySelectorAll('.pane').length === 1)
    await page.waitForTimeout(1000)
    await save(page, 'terminals-narrow.webp', '420x560 at 2x')
    await page.context().close()

    // The floor by itself: every room of the demo, to its end.
    page = await office(1240, 800)
    const tall = await page.evaluate(() => {
      const floor = document.querySelector('.floor')
      return Math.ceil(innerHeight - floor.clientHeight + floor.scrollHeight)
    })
    await page.setViewportSize({ width: 1240, height: tall })
    await page.waitForTimeout(600)
    await save(page, 'office.webp', `1240x${tall} at 2x`)

    // One person for each status, cut out of the floor with nothing behind them.
    await page.addStyleTag({ content: 'html, body, #app, .floor, .room, .seat, .desk { background: transparent !important; } .room { border-color: transparent !important; }' })
    const poses = { needs: 1, working: 2, away: 3, done: 4, idle: 6, trouble: 7 }
    for (const [status, n] of Object.entries(poses)) {
      const person = desk(page, n).locator('.scene')
      const box = await person.boundingBox()
      await writeFile(path.join(out, `pose-${status}.webp`), await webp(page, await person.screenshot({ omitBackground: true }), 0.9))
      console.log(`${`pose-${status}.webp`.padEnd(24)} ${Math.round(box.width)}x${Math.round(box.height)} at 2x`)
    }
    await page.context().close()

    // What a link to the site shows: the top of the page itself.
    const context = await browser.newContext({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1, colorScheme: 'dark' })
    page = await context.newPage()
    await page.goto(`${base}?card`)
    await page.waitForFunction(() => document.fonts.status === 'loaded')
    await page.waitForTimeout(500)
    await page.screenshot({ path: path.join(out, 'card.png') })
    console.log(`${'card.png'.padEnd(24)} 1200x630`)
    await context.close()
  } finally {
    await browser.close()
    if (serving) server.close()
  }
}
