// The real programs in the real app: Claude Code and Codex, each started as an
// agent with nothing to do, so they draw their opening screen and wait. Nothing
// is typed into them, so no model is called and nothing is spent.
//
//   npm run tauri build -- --debug --no-bundle
//   node tools/e2e/real-clis.mjs [picture.png]
//
// Two things only the real programs can show are checked:
//
// - In a folder nobody has told them to trust, each stops on its question about
//   the folder, and its desk says it needs you, with no task given.
// - In a folder Codex does trust (this repository), its desk stays idle while its
//   pane is resized: drawing its screen again is not work.
//
// Prints what each terminal shows and saves a picture of the window. A program
// that is not on this computer is skipped. The app runs as the named office `real`.
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { attach, forgetAddress, killTree, launch, root, sleep } from './lib.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const picture = path.resolve(process.argv[2] ?? path.join(root, '.impeccable', 'review', 'real-clis.png'))
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-real-'))
const work = path.join(data, 'demo-project')
mkdirSync(work)
mkdirSync(path.dirname(picture), { recursive: true })
// A stand-in to open beside Codex, so that Codex's pane changes size.
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([{ id: 'fake', name: 'Fake Agent', tag: 'Fake', program: process.execPath, args: [path.join(here, 'fake-agent.mjs')], task: 'last' }]))

let failed = 0
function check(what, ok, detail = '') {
  if (!ok) failed += 1
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${ok || detail === '' ? '' : `  (${detail})`}`)
}

const { child, port } = launch({ port: 9246, env: { MOSHPIT_DATA_DIR: data, MOSHPIT_INSTANCE: 'real' } })
let shown = 0
try {
  const { browser, page } = await attach(port)
  for (let i = 0; i < 40; i++) {
    const box = await page.evaluate(() => { const f = document.querySelector('.floor'); const r = f?.getBoundingClientRect(); return f ? [Math.round(r.width), Math.round(r.height)] : null }).catch(() => null)
    if (box && box[0] > 0 && box[1] > 0) break
    await sleep(500)
  }
  // The programs are looked for as the app starts; give that a moment.
  await sleep(1500)

  const desk = title => page.locator('.floor button[data-desk]', { hasText: title })
  const state = async title => ((await desk(title).getAttribute('class')) ?? '').split(' ').find(word => ['starting', 'working', 'needs_you', 'done', 'idle', 'failed', 'asleep'].includes(word)) ?? ''
  const until = async (ready, ms) => {
    const end = Date.now() + ms
    while (Date.now() < end) {
      if (await ready()) return true
      await sleep(200)
    }
    return false
  }
  /** Seat a program with no task. False when it is not on this computer. */
  const seat = async (program, folder, name) => {
    await page.getByRole('button', { name: 'New agent' }).first().click()
    const choice = page.getByRole('radiogroup').getByText(program, { exact: true })
    if ((await choice.count()) === 0) {
      console.log(`${program}: not found on this computer, skipped`)
      await page.keyboard.press('Escape')
      return false
    }
    await choice.click()
    await page.getByRole('textbox', { name: 'Folder' }).fill(folder)
    await page.getByLabel(/^Name/).fill(name)
    await page.getByRole('button', { name: 'Start agent' }).click()
    await page.locator('.pane', { hasText: name }).waitFor()
    return true
  }
  /** Every state a desk is seen in over a while, in the order first seen. */
  const watch = async (title, ms) => {
    const seen = []
    const end = Date.now() + ms
    while (Date.now() < end) {
      const now = await state(title)
      if (now && !seen.includes(now)) seen.push(now)
      await sleep(100)
    }
    return seen
  }

  // ── a folder nobody trusted: each asks about it, and its desk needs you ──
  for (const name of ['Claude Code', 'Codex']) {
    const title = `${name}, new folder`
    if (!(await seat(name, work, title))) continue
    shown += 1
    check(`${name}, started with no task in a folder it was not told to trust, needs you`, await until(async () => (await state(title)) === 'needs_you', 20000), await state(title))
    check('and its desk says what for', ((await desk(title).locator('.doing').textContent()) ?? '') === 'Asks whether to trust this folder', (await desk(title).locator('.doing').textContent()) ?? '')
  }
  // Let both draw their opening screens.
  await sleep(4000)
  for (const pane of await page.locator('.pane').all()) {
    const title = await pane.locator('h2').innerText()
    const status = await pane.locator('.chip').innerText()
    const text = (await pane.locator('.xterm-accessibility-tree').innerText().catch(() => '')).split('\n').map(line => line.trimEnd()).filter(line => line.trim().length > 0)
    console.log(`\n=== ${title} [${status}]: ${text.length} lines on screen`)
    console.log(text.slice(0, 12).map(line => `  ${line.slice(0, 110)}`).join('\n'))
  }
  await page.screenshot({ path: picture })
  console.log(`\npicture: ${picture}`)
  while ((await page.locator('.pane').count()) > 0) {
    const left = await page.locator('.pane').count()
    await page.locator('.pane').first().getByRole('button', { name: 'Put this terminal away' }).click()
    await until(async () => (await page.locator('.pane').count()) < left, 5000)
  }

  // ── Codex, in a folder it trusts: resizing its pane is not work ──────────
  if (await seat('Codex', root, 'Codex, at rest')) {
    // Its opening screen takes some seconds, with pauses: watched for longer than the office allows one.
    const opening = await watch('Codex, at rest', 36000)
    check('Codex, with nothing to do in a folder it trusts, draws its opening screen and settles as idle', opening.at(-1) === 'idle' && opening.every(seen => seen === 'starting' || seen === 'idle'), opening.join(' > '))
    const during = { beside: [], zoomed: [], back: [] }
    const beside = watch('Codex, at rest', 6000)
    await seat('Fake Agent', work, 'Beside')
    during.beside = await beside
    const zoomed = watch('Codex, at rest', 5000)
    await page.locator('.pane', { hasText: 'Codex, at rest' }).getByRole('button', { name: 'Give this terminal the room' }).click()
    during.zoomed = await zoomed
    const back = watch('Codex, at rest', 5000)
    await page.locator('.pane', { hasText: 'Codex, at rest' }).getByRole('button', { name: 'Put the other terminals back' }).click()
    during.back = await back
    check('it stays idle when a pane opens beside it', !during.beside.includes('working'), during.beside.join(' > '))
    check('and when it is given the room', !during.zoomed.includes('working'), during.zoomed.join(' > '))
    check('and when the others come back', !during.back.includes('working'), during.back.join(' > '))
  }
  await browser.close().catch(() => {})
} catch (error) {
  failed += 1
  console.log(`FAIL the test itself broke: ${error?.stack ?? error}`)
} finally {
  killTree(child.pid)
  forgetAddress('real')
  await sleep(500)
  rmSync(data, { recursive: true, force: true })
}
console.log(failed === 0 ? `\nall passed (${shown} of the real programs shown)` : `\n${failed} failed`)
process.exit(shown > 0 && failed === 0 ? 0 : 1)
