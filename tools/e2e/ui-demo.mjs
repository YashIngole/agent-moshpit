// Checks of the interface on its own, in a headless browser with the pretend office.
// No programs, no desktop shell: this is about what the window does with a keyboard,
// a narrow floor, a new arrival, and any number of terminals.
//
//   npm run build && node tools/e2e/ui-demo.mjs
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'
import packageMetadata from '../../package.json' with { type: 'json' }

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const PORT = Number(process.env.MOSHPIT_UI_PORT || 4174)
const base = `http://localhost:${PORT}/`

const checks = []
const check = (name, ok, detail = '') => checks.push({ name, ok: Boolean(ok), ...(detail !== '' ? { detail: String(detail) } : {}) })
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

async function reachable() {
  try {
    return (await fetch(base)).ok
  } catch {
    return false
  }
}

const vite = path.join(root, 'node_modules', 'vite', 'bin', 'vite.js')
const server = spawn(process.execPath, [vite, 'preview', '--port', String(PORT), '--strictPort'], { cwd: root, stdio: 'ignore' })
for (let i = 0; i < 60 && !(await reachable()); i++) await sleep(250)

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

const browser = await launch()
const errors = []

/** `platform`: the system the window is told it is on, for what differs between them. */
async function open(query, width = 1280, height = 800, platform = '') {
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: 'light' })
  if (platform) await context.addInitScript(name => Object.defineProperty(navigator, 'platform', { get: () => name }), platform)
  // All demo clipboard effects stay in this context, never the system clipboard.
  await context.addInitScript(() => {
    navigator.clipboard.writeText = async () => {}
    navigator.clipboard.write = async () => {}
    navigator.clipboard.readText = async () => ''
    navigator.clipboard.read = async () => []
  })
  const page = await context.newPage()
  page.on('pageerror', error => errors.push(String(error)))
  page.on('console', message => {
    if (message.type() === 'error') errors.push(message.text())
  })
  await page.goto(`${base}?${query}`)
  await page.waitForSelector('.floor button[data-desk]')
  return page
}

const focused = page => page.evaluate(() => document.activeElement?.getAttribute('data-desk') ?? document.activeElement?.className ?? '')
const panes = page => page.locator('.pane').count()
/** Every pane as it is laid out, in reading order: the page keeps them in an order of its own, which never changes. */
const laidOut = page =>
  page.locator('.pane').evaluateAll(all =>
    all
      .map(p => ({ r: p.getBoundingClientRect(), id: p.dataset.pane ?? '', title: p.querySelector('h2')?.textContent ?? '' }))
      .map(({ r, id, title }) => ({ x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), id, title }))
      .sort((a, b) => a.y - b.y || a.x - b.x)
  )
/** The size of every pane, in the order they are laid out. */
const boxes = async page => (await laidOut(page)).map(({ x, y, w, h }) => ({ x, y, w, h }))
/** The names on the panes, in the order they are laid out. */
const paneTitles = async page => (await laidOut(page)).map(pane => pane.title)
const desk = (page, n) => page.locator(`.floor button[data-desk="demo-${n}"]`)
/** A click with Ctrl held (Cmd on a Mac): their terminal in place of the one in use. */
const HELD = { modifiers: ['ControlOrMeta'] }

/** Keep asking until it is true, or give up. */
async function until(fn, ms = 6000) {
  const end = Date.now() + ms
  for (;;) {
    try {
      if (await fn()) return true
    } catch {
      // not there yet
    }
    if (Date.now() > end) return false
    await sleep(80)
  }
}

try {
  // ── the floor and the keyboard ──
  let page = await open('demo=office&still')
  check('the floor offers one tab stop, not one per desk', (await page.locator('.floor button[tabindex="0"]').count()) === 1)
  check('every desk can still be reached', (await page.locator('.floor button[data-desk]').count()) === 7)
  check('a desk on the floor has no × of its own: the menu, Delete and a middle click take it away', (await page.locator('.seat .remove').count()) === 0)
  check('a room seats someone new from the + on its sign, not from an empty desk', (await page.locator('.floor .spare').count()) === 0 && (await page.getByRole('button', { name: 'Add an agent in shop' }).count()) === 1)
  check('a desk says nothing its status already says', (await desk(page, 5).locator('.doing').count()) === 0 && ((await desk(page, 2).locator('.doing').textContent()) ?? '') === 'Moving the session check into middleware')
  await page.keyboard.press('Tab')
  for (let i = 0; i < 8 && !(await focused(page)).startsWith('demo-'); i++) await page.keyboard.press('Tab')
  check('focus reaches the first desk', (await focused(page)) === 'demo-1')
  await page.keyboard.press('ArrowRight')
  check('right arrow walks to the next desk', (await focused(page)) === 'demo-2')
  check('each desk says which program its agent is', ((await desk(page, 2).locator('.tag').textContent()) ?? '') === 'Codex')
  check('someone with a hand up is named on the band', ((await page.locator('.band h2').textContent()) ?? '') === 'Fix the checkout total')

  // ── one terminal, beside the floor ──
  await desk(page, 2).click()
  check('a click on a desk opens its terminal', await until(async () => (await panes(page)) === 1))
  check('the floor is still there beside it', await page.locator('.floor').isVisible())
  check('the desk is marked as open', ((await desk(page, 2).getAttribute('class')) ?? '').includes('open'))
  check('the terminal is drawn', await until(async () => (await page.locator('.pane .xterm').count()) === 1))
  check('and says what its program printed', await until(async () => ((await page.locator('.pane .xterm-accessibility-tree').innerText()) ?? '').includes('Refactor auth middleware')))

  // A click on another desk gives it a pane of its own beside the first, and takes nobody's place.
  await desk(page, 5).click()
  check('a click on another desk opens it beside the first, which stays', await until(async () => (await panes(page)) === 2) && (await page.locator('.pane h2', { hasText: 'Refactor auth middleware' }).count()) === 1)
  check('what a second click does, and how to swap one in, is said the first time', await until(async () => ((await page.locator('.toast').textContent()) ?? '').includes('Click a desk again to put its terminal away')))
  let b = await boxes(page)
  check('two panes sit side by side, evenly', b.length === 2 && b[0].y === b[1].y && Math.abs(b[0].w - b[1].w) <= 2, JSON.stringify(b))
  // The same again on a desk that is open puts its pane away; the one beside it stays, and nobody is stopped.
  await desk(page, 2).click()
  check('a click on an open desk puts its pane away again', await until(async () => (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Docs pass for the API'))
  check('and their program keeps running', ((await desk(page, 2).getAttribute('class')) ?? '').includes('working') && ((await desk(page, 2).getAttribute('aria-current')) ?? '') !== 'true')
  // With Ctrl (Cmd on a Mac) their terminal takes the place of the one in use: the room keeps its shape.
  await desk(page, 6).click(HELD)
  check('Ctrl and a click shows a desk in place of the terminal in use', await until(async () => (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Bump dependencies'))
  await desk(page, 5).click(HELD)
  await until(async () => ((await page.locator('.pane h2').textContent()) ?? '') === 'Docs pass for the API')
  // Enter on a desk opens their terminal or goes to it, and puts nothing away.
  await desk(page, 5).focus()
  await page.keyboard.press('Enter')
  check('Enter on an open desk goes to its terminal and does not put it away', (await panes(page)) === 1 && await until(async () => (await focused(page)).includes('xterm')))
  await desk(page, 2).click()
  check('and a click brings the other back beside it', await until(async () => (await panes(page)) === 2))
  b = await boxes(page)

  // ── dragging the edge between two ──
  const edge = page.locator('.edge.column').first()
  const at = await edge.boundingBox()
  await page.mouse.move(at.x + at.width / 2, at.y + 200)
  await page.mouse.down()
  await page.mouse.move(at.x + at.width / 2 + 160, at.y + 200, { steps: 6 })
  await page.mouse.up()
  const dragged = await boxes(page)
  check('dragging the edge gives one pane room from the other', dragged[0].w > b[0].w + 120 && dragged[1].w < b[1].w - 120, JSON.stringify(dragged))
  check('and nothing else moves', dragged[0].x === b[0].x && dragged[0].w + dragged[1].w === b[0].w + b[1].w)
  await edge.dblclick()
  b = await boxes(page)
  check('a double click makes them even again', Math.abs(b[0].w - b[1].w) <= 2, JSON.stringify(b))

  // ── any number: three, four, and back down ──
  await desk(page, 4).click()
  await until(async () => (await panes(page)) === 3)
  b = await boxes(page)
  check('three panes are two above one', b.filter(p => p.y === b[0].y).length === 2 && b.filter(p => p.y > b[0].y).length === 1, JSON.stringify(b))
  await desk(page, 6).click()
  await until(async () => (await panes(page)) === 4)
  b = await boxes(page)
  check('four panes are two by two', new Set(b.map(p => p.x)).size === 2 && new Set(b.map(p => p.y)).size === 2, JSON.stringify(b))
  check('a terminal that was already open was not made again', (await page.locator('.pane .xterm').count()) === 4)

  await page.getByRole('button', { name: 'Give this terminal the room' }).first().click()
  check('one pane can be given the whole room', await until(async () => (await page.locator('.pane:not(.hidden)').count()) === 1))
  await page.getByRole('button', { name: 'Put the other terminals back' }).click()
  check('and the others come back as they were', await until(async () => (await page.locator('.pane:not(.hidden)').count()) === 4))

  await page.getByRole('button', { name: 'Put this terminal away' }).first().click()
  await until(async () => (await panes(page)) === 3)
  await page.getByRole('button', { name: 'Put this terminal away' }).first().click()
  await page.getByRole('button', { name: 'Put this terminal away' }).first().click()
  check('panes close down to one', await until(async () => (await panes(page)) === 1))

  // ── a program started in a pane that is already there ──
  // It is started at that pane's size and told it again: the pane's size has not changed,
  // so nothing else would ever say it. (A guess at the whole room's width drew it garbled.)
  const sizing = await open('demo=office&still')
  await desk(sizing, 2).click()
  await desk(sizing, 3).click()
  await until(async () => (await panes(sizing)) === 2)
  const awayPane = sizing.locator('.pane[data-pane="demo-3"]')
  const sizeOf = () => awayPane.locator('.term-host').getAttribute('data-size')
  // The floor narrows to the strip as the second pane opens, so the pane is fitted twice.
  let paneSize = ''
  await until(async () => {
    const before = await sizeOf()
    await sleep(200)
    paneSize = (await sizeOf()) ?? ''
    return paneSize !== '' && paneSize === before
  })
  await awayPane.getByRole('button', { name: 'Carry on' }).click()
  await until(() => sizing.evaluate(() => window.__demo.started.length > 0))
  const startedAt = await sizing.evaluate(() => window.__demo.started.map(s => `${s.agent} ${s.cols}x${s.rows}`).join(', '))
  check('a program carried on in a pane is started at the size of that pane', startedAt === `demo-3 ${paneSize}`, `${startedAt} against a pane of ${paneSize}`)
  check('and is told the size again once it runs', await until(() => sizing.evaluate(size => window.__demo.sized.filter(s => s.agent === 'demo-3' && `${s.cols}x${s.rows}` === size).length >= 2, paneSize)), JSON.stringify(await sizing.evaluate(() => window.__demo.sized)))
  await awayPane.getByRole('button', { name: /^More for/ }).click()
  await sizing.getByRole('menuitem', { name: /Restart their program/ }).click()
  await until(() => sizing.evaluate(() => window.__demo.started.length > 1))
  const restartedAt = await sizing.evaluate(() => window.__demo.started.slice(1).map(s => `${s.agent} ${s.cols}x${s.rows}`).join(', '))
  check('a restart from the menu starts it at the size of its pane too', restartedAt === `demo-3 ${paneSize}`, `${restartedAt} against a pane of ${paneSize}`)
  await sizing.context().close()

  // ── the floor and back ──
  await page.keyboard.press('Control+Backquote')
  check('Ctrl+` goes back to the floor', await until(async () => (await panes(page)) === 0))
  check('and offers the terminals again', (await page.getByRole('button', { name: /terminals/i }).count()) === 1)
  await page.keyboard.press('Control+Backquote')
  check('Ctrl+` again brings them back', await until(async () => (await panes(page)) === 1))
  await page.getByRole('button', { name: /hide terminals/i }).click()
  await until(async () => (await panes(page)) === 0)

  // ── someone waiting ──
  await page.getByRole('button', { name: 'Open their terminal' }).click()
  check('the band opens the terminal of whoever is waiting', await until(async () => (await page.locator('.pane h2', { hasText: 'Fix the checkout total' }).count()) === 1))
  check('the waiting cue stays until the question is answered, as a chip in the bar', (await page.locator('.bar .attention').count()) === 1)
  check('and the band leaves the terminals where they are', (await page.locator('.band').count()) === 0)
  await page.keyboard.press('Control+Backquote')
  check('back on the floor, it does again', await until(async () => (await page.locator('.band').count()) === 1))

  // ── a half-written task is not lost ──
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByLabel(/What should they do/).fill('Half a thought')
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'New agent' }).first().click()
  check('a half-written task is still there when the form is opened again', (await page.getByLabel(/What should they do/).inputValue()) === 'Half a thought')
  check('programs on this computer are offered first', (await page.getByRole('radio').count()) === 5 && (await page.getByRole('radio', { name: /OpenCode/ }).count()) === 0)
  await page.getByRole('button', { name: '+2 to install' }).click()
  check('and ones the office can install on asking', (await page.getByRole('radio').count()) === 7 && (await page.getByRole('radio', { name: /OpenCode/ }).count()) === 1)
  check('a program installed its own way is named, not offered', (await page.getByRole('radio', { name: /Cursor/ }).count()) === 0 && (await page.getByText(/installed their own way: Cursor CLI/).count()) === 1)
  await page.getByRole('radiogroup').getByText('Hermes', { exact: true }).click()
  check('a program that takes its task in its terminal is not asked for one here', (await page.getByLabel(/What should they do/).count()) === 0)
  await page.context().close()

  // ── a program that is not here yet: installed, then started ──
  page = await open('demo=office&still')
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('button', { name: /to install$/ }).click()
  await page.getByRole('radiogroup').getByText('OpenCode').click()
  check('what installs it is shown before it runs', (await page.locator('code.line').textContent()) === 'npm install -g opencode-ai')
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Install OpenCode and start' }).click()
  check('the install runs in a pane of its own', await until(async () => (await page.locator('.pane[aria-label*="OpenCode"]').count()) === 1))
  check('and then the agent takes its place', await until(async () => ((await page.locator('.pane h2').first().textContent()) ?? '') === 'OpenCode in shop'))
  check('which leaves no install pane behind', (await page.locator('.pane[aria-label*="installing"], .pane[aria-label*="installed"]').count()) === 0)
  check('and a desk for them on the floor', ((await page.locator('.floor button[data-desk^="demo-new-"] .tag').textContent()) ?? '') === 'OpenCode')

  // ── programs, their versions, and updates ──
  check('the menu shows that updates are out', (await page.locator('.more .news').count()) === 1)
  await page.getByRole('button', { name: 'More', exact: true }).click()
  check('and says which', ((await page.getByRole('menuitem', { name: /Agent programs/ }).textContent()) ?? '').includes('Claude Code, Gemini CLI'))
  await page.getByRole('menuitem', { name: /Agent programs/ }).click()
  const gemini = page.locator('aside.panel li', { hasText: 'Gemini CLI' })
  check('a program that is behind says which version is out', ((await gemini.locator('.version').textContent()) ?? '').includes('0.63.0'))
  check('and what updates it', ((await gemini.locator('code').textContent()) ?? '') === 'npm install -g @google/gemini-cli@latest')
  await gemini.getByRole('button', { name: 'update' }).click()
  check('the update runs in a pane of its own', await until(async () => (await page.locator('.pane[aria-label*="Gemini CLI"]').count()) === 1))
  check('and says when it is done', await until(async () => ((await page.locator('.pane[aria-label*="Gemini CLI"] h2').textContent()) ?? '').includes('updated')))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  check('after which it is no longer said to be behind', ((await page.getByRole('menuitem', { name: /Agent programs/ }).textContent()) ?? '').match(/1 update out: Claude Code$/) !== null)
  await page.keyboard.press('Escape')
  await page.locator('.pane[aria-label*="Gemini CLI"]').getByRole('button', { name: 'Put this terminal away' }).click()
  check('a finished update is put away for good', await until(async () => (await page.locator('.pane[aria-label*="Gemini CLI"]').count()) === 0))
  await page.context().close()

  // ── a narrow window ──
  page = await open('demo=office&still', 390, 760)
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 1)
  check('in a narrow window the terminal takes the room', !(await page.locator('.floor').isVisible()))
  await page.keyboard.press('Control+Backquote')
  check('and the floor comes back when it is put away', await until(() => page.locator('.floor').isVisible()))
  await page.context().close()

  // ── a new arrival ──
  page = await open('demo=calm')
  check('nobody walks in when the window opens', (await page.locator('.floor button.arriving').count()) === 0)
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByLabel(/What should they do/).fill('Tidy the changelog')
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Start agent' }).click()
  const fresh = page.locator('.floor button[data-desk^="demo-new-"]')
  await fresh.waitFor()
  check('a new agent gets a desk that plays the walk-in', ((await fresh.getAttribute('class')) ?? '').includes('arriving'))
  check('and a terminal of their own', await until(async () => ((await page.locator('.pane h2').first().textContent()) ?? '') === 'Tidy the changelog'))
  check('the walk-in plays once and is then over', await until(async () => !((await fresh.getAttribute('class')) ?? '').includes('arriving'), 3000))
  await page.context().close()

  // ── a crowd ──
  page = await open('demo=crowd&still', 1440, 900)
  const desks = page.locator('.floor button[data-desk]')
  await desks.nth(0).click()
  for (const n of [1, 2, 3, 6, 8]) await desks.nth(n).click()
  await until(async () => (await panes(page)) === 6)
  b = await boxes(page)
  check('six panes are three by two', new Set(b.map(p => p.x)).size === 3 && new Set(b.map(p => p.y)).size === 2, JSON.stringify(b))
  check('and each is still wide enough to read', Math.min(...b.map(p => p.w)) >= 300, Math.min(...b.map(p => p.w)))
  await page.context().close()

  // ── what a vibe coder reaches for every day (docs/reviews/ux-sweep-2026-10-08.md) ──
  page = await open('demo=office&still')
  await desk(page, 2).click()
  await desk(page, 4).click()
  await desk(page, 5).click()
  await until(async () => (await panes(page)) === 3)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 6).click(HELD)
  check('swapping someone in from the floor brings the grid back, with them in it', await until(async () => (await panes(page)) === 3 && (await page.locator('.pane h2', { hasText: 'Bump dependencies' }).count()) === 1))
  // A pane is there a moment before its terminal is, and the terminal takes the keyboard
  // when it comes: a desk focused in between would lose it again.
  await until(async () => (await focused(page)).includes('xterm'))
  await desk(page, 1).focus()
  await page.keyboard.press('Control+Enter')
  check('Ctrl+Enter on a desk opens it beside the others', await until(async () => (await panes(page)) === 4), `${await panes(page)} panes, keyboard in ${await focused(page)}`)

  const spot = title => page.locator('.pane', { hasText: title }).evaluate(p => `${Math.round(p.getBoundingClientRect().x)},${Math.round(p.getBoundingClientRect().y)}`)
  const [one, two] = await paneTitles(page)
  const [oneAt, twoAt] = [await spot(one), await spot(two)]
  const head = await page.locator('.pane', { hasText: one }).locator('.status').boundingBox()
  const there = await page.locator('.pane', { hasText: two }).locator('.screen').boundingBox()
  await page.mouse.move(head.x + 4, head.y + 4)
  await page.mouse.down()
  await page.mouse.move(there.x + there.width / 2, there.y + there.height / 2, { steps: 10 })
  await page.mouse.up()
  check('a pane dragged by its header onto another changes places with it', await until(async () => (await spot(one)) === twoAt && (await spot(two)) === oneAt), `${one} ${await spot(one)} / ${two} ${await spot(two)}`)

  await desk(page, 3).click({ button: 'right' })
  check('a right-click on a desk opens its menu', await until(async () => (await page.getByRole('menuitem', { name: /Remove this desk/ }).count()) === 1))
  check('which renames, carries on, shows and copies the folder', (await page.getByRole('menuitem').allTextContents()).join('|').match(/Rename.*Carry on.*Show their folder.*Copy the folder path/) !== null)
  await page.keyboard.press('Escape')
  check('Escape closes it', await until(async () => (await page.locator('.menu-sheet').count()) === 0))
  await desk(page, 3).click(HELD)
  check('opening an away desk shows them as they were left, without starting them', await until(async () => (await page.locator('.pane .ended').count()) >= 1) && ((await desk(page, 3).getAttribute('class')) ?? '').includes('asleep'))
  await desk(page, 3).click({ button: 'middle' })
  check('a middle click takes a desk off the floor', await until(async () => (await desk(page, 3).count()) === 0))
  check('with a moment to bring it back', (await page.getByRole('button', { name: 'Undo' }).count()) === 1)
  await page.getByRole('button', { name: 'Undo' }).click()
  check('and Undo does', await until(async () => (await desk(page, 3).count()) === 1))

  await desk(page, 6).focus()
  await page.keyboard.press('F2')
  await page.getByRole('dialog', { name: /Rename/ }).getByRole('textbox').fill('Bump the deps')
  await page.keyboard.press('Enter')
  check('F2 on a desk renames it', await until(async () => ((await desk(page, 6).locator('.name').textContent()) ?? '') === 'Bump the deps'))

  await page.locator('.bar .count', { hasText: 'in trouble' }).click()
  check('a count in the bar finds the desk it counts', await until(async () => (await focused(page)) === 'demo-7'))

  await page.locator('.pane .term-host').first().click()
  await page.keyboard.press('Control+Equal')
  check('Ctrl+= makes the terminals’ text bigger, and it is remembered', await until(async () => (await page.evaluate(() => JSON.parse(localStorage.getItem('moshpit.view') ?? '{}').font)) === 14))
  await page.keyboard.press('Control+0')
  await page.keyboard.press('Control+Shift+KeyF')
  check('Ctrl+Shift+F finds in a terminal', await until(async () => (await page.getByRole('searchbox').count()) + (await page.locator('.find input').count()) > 0))
  // The field takes the keyboard on the next frame, which a browser with no screen can be slow to draw.
  check('and takes the keyboard from the terminal', await until(() => page.evaluate(() => document.activeElement?.matches('.find input') ?? false)))
  await page.keyboard.type('src')
  check('and says when it has found it', (await page.locator('.find input').inputValue()) === 'src' && ((await page.locator('.find .hint').textContent()) ?? '') !== 'not found')
  await page.keyboard.press('Escape')
  check('Escape closes the find bar', await until(async () => (await page.locator('.find').count()) === 0))

  const target = page.locator('.pane').first()
  const targetTitle = (await target.locator('h2').textContent()) ?? ''
  await target.locator('.screen').evaluate(screen => {
    const box = screen.getBoundingClientRect()
    const files = new DataTransfer()
    files.items.add(new File(['x'], 'screenshot 1.png', { type: 'image/png' }))
    const at = { bubbles: true, cancelable: true, dataTransfer: files, clientX: box.x + 40, clientY: box.y + 40 }
    screen.dispatchEvent(new DragEvent('dragover', at))
    screen.dispatchEvent(new DragEvent('drop', at))
  })
  check('a file dropped on a pane is pasted into it as its path', await until(async () => ((await page.locator('.pane', { hasText: targetTitle }).locator('.xterm-accessibility-tree').innerText()) ?? '').replace(/\n/g, '').includes('"C:/Users/you/Desktop/screenshot 1.png"')), JSON.stringify(await page.evaluate(() => window.__demo.typed.slice(-3))))

  // Two terminals or more: the strip of names unless a width was chosen.
  if ((await page.locator('.seat.listed').count()) === 0) await page.locator('.grip').dblclick()
  check('the strip beside the terminals is a list of names', await until(async () => (await page.locator('.seat.listed').count()) > 0))
  check('with nothing cut to a few letters', await page.locator('.seat.listed .name').evaluateAll(names => names.every(n => n.scrollWidth <= n.clientWidth + 1)))

  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: /Agent programs/ }).click()
  // Measured once the panel has slid in.
  check('the Agent programs panel fits its own buttons', await until(() => page.locator('aside.panel .do .button').evaluateAll(buttons => buttons.length > 0 && buttons.every(b => b.getBoundingClientRect().right <= innerWidth))))
  await page.context().close()

  page = await open('demo=crowd&still', 1440, 900)
  const inView = await page.locator('.floor button.desk').evaluateAll(desks => desks.filter(d => d.getBoundingClientRect().bottom <= innerHeight).length)
  check('a crowd of 20 is drawn smaller, most of it on one screen', inView >= 12, `${inView} of 20 in view`)
  await page.context().close()

  page = await open('demo=office&still', 420, 760)
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 1)
  check('a narrow window still reaches the menu', ((await page.getByRole('button', { name: 'More', exact: true }).boundingBox())?.x ?? 999) + 32 <= 420)
  check('and whoever needs you is a chip that keeps the bar one line', (await page.locator('.bar .attention').count()) === 1 && ((await page.locator('.bar').boundingBox())?.height ?? 999) <= 53)
  await page.context().close()

  // ── the second sweep (docs/reviews/ux-sweep-2026-10-09.md) ──
  page = await open('demo=office&still')
  const typed = () => page.evaluate(() => window.__demo.typed.map(t => t.data).join(''))
  await desk(page, 1).click()
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 2)
  const claude = page.locator('.pane', { hasText: 'Fix the checkout total' })
  await claude.locator('.term-host').click()
  await page.keyboard.press('Shift+Tab')
  check('Shift+Tab stays in the terminal', await page.evaluate(() => document.activeElement?.closest('.term-host') !== null), await page.evaluate(() => document.activeElement?.outerHTML.slice(0, 80)))
  check('and reaches the program as ESC [ Z', (await typed()).includes('\x1b[Z'))
  await page.keyboard.type('n')
  check('so the next key is the program’s too, not a shortcut of the floor', (await typed()).endsWith('n') && (await page.locator('aside.panel').count()) === 0)

  // Paste from the right-click menu reads the clipboard itself: text as text, a picture as a path.
  await page.evaluate(() => {
    window.__clip = [new ClipboardItem({ 'text/plain': new Blob(['from the clipboard'], { type: 'text/plain' }) })]
    navigator.clipboard.read = async () => window.__clip
  })
  await claude.locator('.term-host').click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Paste/ }).click()
  check('the terminal menu’s Paste pastes the clipboard’s text', await until(async () => (await typed()).includes('from the clipboard')))
  await page.evaluate(() => {
    const png = Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
    window.__clip = [new ClipboardItem({ 'image/png': new Blob([png], { type: 'image/png' }) })]
  })
  await claude.locator('.term-host').click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Paste/ }).click()
  check('and a picture the way Ctrl+V does, as the path of a file holding it', await until(async () => (await typed()).includes('pasted-1.png')))

  // A double-click anywhere on a header gives it the room, the title included.
  await claude.locator('h2').dblclick()
  check('a double-click on a pane’s title gives it the room', await until(async () => (await page.locator('.pane:not(.hidden)').count()) === 1))
  check('rather than renaming it', (await page.getByRole('dialog', { name: /Rename/ }).count()) === 0)
  await claude.locator('header').dblclick({ position: { x: 300, y: 20 } })
  check('and another puts the others back', await until(async () => (await page.locator('.pane:not(.hidden)').count()) === 2))
  await claude.getByRole('button', { name: /^More for/ }).click()
  check('the menu says how to rename: F2', ((await page.getByRole('menuitem', { name: /^Rename/ }).textContent()) ?? '').includes('F2') && !((await page.getByRole('menuitem', { name: /^Rename/ }).textContent()) ?? '').includes('double-click'))

  // File paths open in the editor the person has, which is named.
  check('the desk menu opens the folder in the editor that is here', (await page.getByRole('menuitem', { name: 'Open the folder in Cursor' }).count()) === 1)
  await page.getByRole('menuitem', { name: 'Open the folder in Cursor' }).click()
  check('in that editor', await until(async () => (await page.evaluate(() => window.__demo.opened.at(-1)?.editor)) === 'cursor'))
  const row = await claude.locator('.xterm-accessibility-tree > div', { hasText: 'src/cart/total.ts' }).first().boundingBox()
  const screenBox = await claude.locator('.xterm-screen').boundingBox()
  let onLink = false
  for (let x = screenBox.x + 2; x < screenBox.x + screenBox.width && !onLink; x += 3) {
    await page.mouse.move(x, row.y + row.height / 2)
    onLink = (await claude.locator('.xterm-cursor-pointer').count()) > 0
  }
  check('a file path in a terminal is a link', onLink)
  await page.mouse.down()
  await page.mouse.up()
  check('a plain click on it says which editor Ctrl and a click opens', await until(async () => ((await page.locator('.toast').textContent()) ?? '').includes('open it in Cursor at that line')))
  const toast = await page.locator('.toast').boundingBox()
  const floorBox = await page.locator('.floor').boundingBox()
  check('which is said over the floor, not over the terminals’ last lines', toast.x >= floorBox.x && toast.x + toast.width <= floorBox.x + floorBox.width + 1, JSON.stringify({ toast, floorBox }))
  // On a Mac the key is Command: Control and a click is that system's right-click.
  await page.keyboard.down('ControlOrMeta')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('ControlOrMeta')
  check('Ctrl and a click opens the file in that editor, at its line', await until(async () => JSON.stringify(await page.evaluate(() => window.__demo.opened.at(-1))) === JSON.stringify({ agent: 'demo-1', path: 'src/cart/total.ts', line: null, editor: 'cursor' })), JSON.stringify(await page.evaluate(() => window.__demo.opened)))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: /Agent programs/ }).click()
  check('the Agent programs panel offers the editors that are here', (await page.getByRole('radiogroup', { name: 'Files open in' }).getByRole('radio').count()) === 2)
  await page.getByRole('radiogroup', { name: 'Files open in' }).getByText('Zed').click()
  check('and remembers the one picked', await until(async () => (await page.evaluate(() => JSON.parse(localStorage.getItem('moshpit.view') ?? '{}').editor)) === 'zed'))
  await page.keyboard.press('Escape')
  await page.context().close()

  // A click on a notification: the window opens with that desk's terminal.
  page = await open('demo=office&still&open=demo-4')
  check('a clicked notification opens that desk’s terminal', await until(async () => (await page.locator('.pane h2', { hasText: 'Type the orders API' }).count()) === 1))
  // The strip gathers whoever wants a look above the rooms, and says which program each row is.
  await desk(page, 2).click()
  check('two terminals bring the strip of names by themselves', await until(async () => (await page.locator('.seat.listed').count()) > 0))
  const waitingRows = await page.locator('.room.waiting .desk').evaluateAll(rows => rows.map(r => r.className.split(' ').find(c => ['needs_you', 'failed', 'done'].includes(c))))
  check('the strip puts a Waiting group above the rooms: needs you, then trouble, then done', JSON.stringify(waitingRows) === JSON.stringify(['needs_you', 'failed']) || JSON.stringify(waitingRows) === JSON.stringify(['needs_you', 'failed', 'done']), JSON.stringify(waitingRows))
  check('above every room', await page.evaluate(() => document.querySelector('.floor .room')?.classList.contains('waiting')))
  check('and every row in the strip names its program', await page.locator('.seat.listed').evaluateAll(seats => seats.every(s => { const tag = s.querySelector('.tag'); return tag && getComputedStyle(tag).display !== 'none' && tag.textContent.trim().length > 0 })))
  await page.context().close()

  // Twenty agents on a 1280 by 800 screen.
  page = await open('demo=crowd&still', 1280, 800)
  const crowd = await page.evaluate(() => {
    const desks = [...document.querySelectorAll('.floor button.desk')]
    const rooms = [...document.querySelectorAll('.floor .room')].map(r => Math.round(r.getBoundingClientRect().top))
    const statuses = [...document.querySelectorAll('.floor .status')]
    return {
      inView: desks.filter(d => d.getBoundingClientRect().bottom <= innerHeight).length,
      widths: new Set(desks.map(d => Math.round(d.getBoundingClientRect().width))).size,
      paired: rooms.length - new Set(rooms).size,
      poking: statuses.filter(s => [...s.children].some(c => getComputedStyle(c).display !== 'none' && c.getBoundingClientRect().right > s.getBoundingClientRect().right + 1)).length,
      cutTimes: [...document.querySelectorAll('.floor .since')].filter(t => getComputedStyle(t).display !== 'none' && t.scrollWidth > t.clientWidth + 1).length
    }
  })
  check('a crowd of 20 fits a 1280 by 800 screen', crowd.inView === 20, JSON.stringify(crowd))
  check('two rooms of four share a row', crowd.paired >= 2, JSON.stringify(crowd))
  check('every desk in a crowd is drawn the same size, a room of one included', crowd.widths === 1, JSON.stringify(crowd))
  check('nothing on a desk’s status line runs past it, and no time is cut short', crowd.poking === 0 && crowd.cutTimes === 0, JSON.stringify(crowd))
  await page.context().close()

  // The small batch.
  page = await open('demo=office&still')
  check('an away desk says away, and nothing more', (await desk(page, 3).locator('.doing').count()) === 0 && ((await desk(page, 3).locator('.chip').textContent()) ?? '').trim() === 'Away')
  check('and its label promises nothing opening does not do', !/start again|carry on/i.test((await desk(page, 3).getAttribute('aria-label')) ?? ''))
  await desk(page, 2).click({ button: 'right' })
  check('every hint in a desk’s menu can be read whole', await page.locator('.menu-sheet [role="menuitem"] span').evaluateAll(hints => hints.length > 0 && hints.every(h => h.scrollHeight <= h.clientHeight + 1 && h.scrollWidth <= h.clientWidth + 1)))
  await page.getByRole('menuitem', { name: /^Stop their program/ }).click()
  check('stopping someone at work asks first', await until(async () => (await page.getByRole('button', { name: 'Stop it' }).count()) === 1))
  check('and does not stop them yet', ((await desk(page, 2).getAttribute('class')) ?? '').includes('working'))
  await page.keyboard.press('Escape')
  await desk(page, 2).click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Restart their program/ }).click()
  check('so does restarting them', await until(async () => (await page.getByRole('button', { name: 'Restart it' }).count()) === 1))
  await page.keyboard.press('Escape')
  await desk(page, 6).click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Stop their program/ }).click()
  check('someone idle is stopped without a question', await until(async () => ((await desk(page, 6).getAttribute('class')) ?? '').includes('asleep')))
  await desk(page, 4).focus()
  await page.keyboard.press('F2')
  await page.getByRole('dialog', { name: /Rename/ }).getByRole('textbox').fill('A name of mine')
  await page.keyboard.press('Enter')
  await until(async () => ((await desk(page, 4).locator('.name').textContent()) ?? '') === 'A name of mine')
  await desk(page, 4).focus()
  await page.keyboard.press('F2')
  await page.getByRole('dialog', { name: /Rename/ }).getByRole('textbox').fill('')
  await page.keyboard.press('Enter')
  check('a name emptied goes back to the one it was given', await until(async () => ((await desk(page, 4).locator('.name').textContent()) ?? '') === 'Type the orders API'))
  await desk(page, 4).focus()
  await page.keyboard.press('Delete')
  check('Delete on a desk hands the keyboard to the desk beside it', await until(async () => (await focused(page)) === 'demo-5'), await focused(page))
  await page.keyboard.press('ArrowLeft')
  check('so the arrow keys still walk the floor', (await focused(page)).startsWith('demo-'), await focused(page))
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Claude Code').click()
  await page.getByLabel(/What should they do/).fill('Make the checkout total right when a coupon is applied')
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Start agent' }).click()
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByLabel(/What should they do/).fill('Make the checkout total right when a coupon is applied')
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Start agent' }).click()
  check('the demo names agents as the app does: from the task, numbered when taken', await until(async () => JSON.stringify(await page.locator('.floor button[data-desk^="demo-new-"] .name').allTextContents()) === JSON.stringify(['Make the checkout total right', 'Make the checkout total right 2'])), JSON.stringify(await page.locator('.floor button[data-desk^="demo-new-"] .name').allTextContents()))
  await page.context().close()

  page = await open('demo=office&still&problem')
  const strip = page.locator('.problem-strip')
  check('a broken harnesses.json says where the file is', ((await strip.textContent()) ?? '').includes('agentmoshpit/harnesses.json'))
  check('and only gives the backslash tip for a backslash', !((await strip.textContent()) ?? '').includes('is written'))
  await strip.getByRole('button', { name: 'Open harnesses.json' }).click()
  check('and opens it in the editor, at the line', await until(async () => JSON.stringify(await page.evaluate(() => window.__demo.opened.at(-1))) === JSON.stringify({ agent: null, path: 'C:/Users/you/AppData/Roaming/io.github.yashingole.agentmoshpit/harnesses.json', line: 3, editor: 'cursor' })))
  await page.context().close()

  // The next list.
  page = await open('demo=office&still')
  check('a finished desk with something new in its terminal wears a dot', (await desk(page, 4).locator('.unread').count()) === 1 && (await desk(page, 3).locator('.unread').count()) === 0)
  check('a desk at work, which prints all the time, does not', (await desk(page, 5).locator('.unread').count()) === 0)
  await desk(page, 4).click()
  await until(async () => (await panes(page)) === 1)
  await page.keyboard.press('Control+Backquote')
  check('and loses it once its terminal has been looked at', await until(async () => (await desk(page, 4).locator('.unread').count()) === 0))
  await desk(page, 2).click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Start another like this/ }).click()
  check('Start another like this opens the form on the same program', await until(async () => (await page.getByRole('radio', { name: 'Codex' }).isChecked().catch(() => false)) === true))
  check('in the same folder', (await page.getByRole('textbox', { name: 'Folder' }).inputValue()) === 'C:/code/shop')
  await page.keyboard.press('Escape')

  // The scroll position survives a trip to the floor.
  await desk(page, 2).click(HELD)
  await until(async () => (await panes(page)) === 1)
  const pane2 = page.locator('.pane').first()
  await pane2.locator('.term-host').click()
  for (let i = 1; i <= 80; i++) await page.keyboard.type(`line ${i}\r`, { delay: 0 })
  const viewport = pane2.locator('.xterm-viewport')
  const box2 = await viewport.boundingBox()
  await page.mouse.move(box2.x + box2.width / 2, box2.y + box2.height / 2)
  await page.mouse.wheel(0, -900)
  await until(async () => (await viewport.evaluate(v => v.scrollTop)) < (await viewport.evaluate(v => v.scrollHeight - v.clientHeight)) - 50)
  const firstRow = () => pane2.locator('.xterm-accessibility-tree > div').first().textContent()
  const before = await firstRow()
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 1)
  check('a terminal scrolled back is where it was after a trip to the floor', await until(async () => (await firstRow()) === before), `${before} / ${await firstRow()}`)

  // Ctrl and the wheel over a terminal: bigger or smaller text.
  await page.mouse.move(box2.x + box2.width / 2, box2.y + box2.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.wheel(0, -100)
  await page.keyboard.up('Control')
  check('Ctrl and the wheel make the terminals’ text bigger', await until(async () => (await page.evaluate(() => JSON.parse(localStorage.getItem('moshpit.view') ?? '{}').font)) === 14))
  await page.keyboard.press('Control+0')

  // Copy on select, when it is turned on.
  await page.evaluate(() => {
    window.__copied = []
    navigator.clipboard.writeText = async text => void window.__copied.push(text)
  })
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitemcheckbox', { name: /Copy on select/ }).click()
  const lineRow = await pane2.locator('.xterm-accessibility-tree > div', { hasText: 'line' }).last().boundingBox()
  await page.mouse.move(lineRow.x + 4, lineRow.y + lineRow.height / 2)
  await page.mouse.down()
  await page.mouse.move(lineRow.x + 60, lineRow.y + lineRow.height / 2, { steps: 4 })
  await page.mouse.up()
  check('with copy on select, selecting text in a terminal copies it', await until(async () => (await page.evaluate(() => window.__copied.length)) > 0))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitemcheckbox', { name: /Closing the window quits/ }).click()
  await page.getByRole('button', { name: 'More', exact: true }).click()
  check('closing the window can be made to quit', (await page.getByRole('menuitemcheckbox', { name: /Closing the window quits/ }).getAttribute('aria-checked')) === 'true' && ((await page.locator('.about').textContent()) ?? '').startsWith('agent moshpit'))
  await page.keyboard.press('Escape')

  // The +N chip lists the terminals behind and switches to one.
  await desk(page, 4).click()
  await desk(page, 6).click()
  await until(async () => (await panes(page)) === 3)
  await page.locator('.pane', { hasText: 'Refactor auth middleware' }).getByRole('button', { name: 'Give this terminal the room' }).click()
  await page.getByRole('button', { name: /more terminals behind this one/ }).click()
  check('the +N chip lists the terminals behind the one given the room', JSON.stringify(await page.getByRole('menuitem').evaluateAll(items => items.map(i => i.firstChild.textContent.trim()))) === JSON.stringify(['Type the orders API', 'Bump dependencies', 'Put them all back']))
  await page.getByRole('menuitem', { name: /^Bump dependencies/ }).click()
  check('and a click switches to that one', await until(async () => ((await page.locator('.pane:not(.hidden) h2').textContent()) ?? '') === 'Bump dependencies'))
  await page.context().close()

  page = await open('demo=office&still')
  await desk(page, 2).click()
  await desk(page, 4).click()
  await page.locator('.pane', { hasText: 'Refactor auth middleware' }).getByRole('button', { name: 'Give this terminal the room' }).click()
  await page.keyboard.press('Alt+2')
  check('Alt+2 while zoomed reports only the newly visible terminal to the core', await until(async () => JSON.stringify(await page.evaluate(() => window.__demo.watched)) === '["demo-4"]'))
  await page.keyboard.press('Control+Shift+BracketLeft')
  check('stepping between zoomed panes updates what the core considers watched', await until(async () => JSON.stringify(await page.evaluate(() => window.__demo.watched)) === '["demo-2"]'))
  await page.context().close()

  // A tray request queued before the window subscribes is consumed on startup.
  page = await open('demo=office&still&new-agent')
  check('a pending tray request opens New agent in a newly created window', await until(async () => (await page.getByRole('button', { name: 'Start agent' }).count()) === 1))
  await page.keyboard.press('Escape')
  const more = page.getByRole('button', { name: 'More', exact: true })
  await more.focus()
  await page.keyboard.press('Enter')
  const activeMenuText = () => page.evaluate(() => document.activeElement?.textContent?.trim() ?? '')
  check('opening More moves focus to its first enabled item', (await activeMenuText()).startsWith('Agent programs'))
  await page.keyboard.press('End')
  check('End reaches Quit', (await activeMenuText()).startsWith('Quit Agent Moshpit'))
  await page.keyboard.press('ArrowDown')
  check('ArrowDown wraps to the first menu item', (await activeMenuText()).startsWith('Agent programs'))
  await page.keyboard.press('ArrowUp')
  check('ArrowUp wraps to the last menu item', (await activeMenuText()).startsWith('Quit Agent Moshpit'))
  await page.keyboard.press('Home')
  check('Home reaches the first menu item', (await activeMenuText()).startsWith('Agent programs'))
  await page.keyboard.press('Escape')
  check('Escape returns focus to More', await more.evaluate(button => document.activeElement === button))
  await page.context().close()

  // The flow extras.
  page = await open('demo=office&still')
  const counted = async () => (await page.locator('.bar .count').allTextContents()).map(t => t.trim()).join(' | ')
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Claude Code').click()
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Start agent' }).click()
  await until(async () => ((await page.locator('.floor button[data-desk^="demo-new-"]').getAttribute('class')) ?? '').includes('starting'))
  check('the bar counts only what wants a look, never who is starting or at work', /^(\d+ (done|in trouble)( \| )?)+$/.test(await counted()), await counted())
  await page.locator('.pane').getByRole('button', { name: 'Put this terminal away' }).click()
  await until(async () => (await panes(page)) === 0)

  // A grid of two, then the floor, then a third desk swapped in: one pane makes room, and says so.
  await desk(page, 2).click()
  await desk(page, 4).click()
  await until(async () => (await panes(page)) === 2)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 6).click(HELD)
  const titles = () => page.locator('.pane h2').allTextContents()
  await until(async () => (await titles()).includes('Bump dependencies'))
  const made = ((await page.locator('.toast').textContent()) ?? '').trim()
  check('a desk swapped in from the floor says which pane made room', /made room for Bump dependencies/.test(made), made)
  const gone = made.split(' made room')[0]
  await page.locator('.toast').getByRole('button', { name: 'Undo' }).click()
  check('and Undo puts it back', await until(async () => (await titles()).includes(gone) && !(await titles()).includes('Bump dependencies')), JSON.stringify(await titles()))

  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await page.keyboard.press('?')
  await page.locator('aside.panel dl').first().waitFor()
  const columns = await page.evaluate(() => {
    const lefts = selector => [...new Set([...document.querySelectorAll(selector)].map(el => Math.round(el.getBoundingClientRect().left)))]
    return { keys: lefts('aside.panel dt'), whats: lefts('aside.panel dd'), text: document.querySelector('aside.panel')?.textContent ?? '' }
  })
  check('the Keys sheet lines its keys up in one column and what they do in another', columns.keys.length === 1 && columns.whats.length === 1, JSON.stringify([columns.keys, columns.whats]))
  check('and lists Ctrl+Q, and Ctrl and a click on a file path with the editor’s name', columns.text.includes('Ctrl+Q') && /(Ctrl|Cmd) and a click on a file path\s*Open it in Cursor, at its line/.test(columns.text) && /Click a desk\s*Open their terminal beside the others, or put it away again/.test(columns.text))
  await page.context().close()

  page = await open('demo=office&still&noeditor')
  await desk(page, 1).click({ button: 'right' })
  check('with no editor here, the desk menu offers none', (await page.getByRole('menuitem', { name: /Open the folder in/ }).count()) === 0)
  await page.keyboard.press('Escape')
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 1)
  await page.locator('.pane').getByRole('button', { name: 'Put this terminal away' }).click()
  check('the hint after putting a pane away points at Stop', await until(async () => ((await page.locator('.toast').textContent()) ?? '').includes('→ Stop')))
  await page.context().close()

  // ── a newer version of the office itself ──
  page = await open('demo=office&still')
  await page.getByRole('button', { name: 'More', exact: true }).click()
  check('with nothing newer out, the menu offers no update of the office', (await page.getByRole('menuitem', { name: /Update to/ }).count()) === 0)
  await page.context().close()

  page = await open('demo=office&still&update')
  await page.getByRole('button', { name: 'More', exact: true }).click()
  const offer = page.getByRole('menuitem', { name: /Update to 99\.0\.0/ })
  check('a newer version of the office is offered first in the menu, beside the one you have', (await page.getByRole('menuitem').first().textContent())?.includes('Update to 99.0.0') && ((await offer.textContent()) ?? '').includes(`You have ${packageMetadata.version}`))
  await offer.click()
  check('choosing it asks the core for the update, once', await until(async () => (await page.evaluate(() => window.__demo.updates)) === 1))
  check('and a "not now" is said, with the offer left in the menu', await until(async () => ((await page.locator('.toast').textContent()) ?? '').includes('Not updated')))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  check('where it still is', (await page.getByRole('menuitem', { name: /Update to 99\.0\.0/ }).count()) === 1)
  await page.context().close()

  page = await open('demo=office&still&update=fails')
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: /Update to 99\.0\.0/ }).click()
  check(
    'an update that cannot be had says why, and where else to get it',
    await until(async () => {
      const said = (await page.locator('.toast').textContent()) ?? ''
      return ['could not be fetched', 'Nothing was changed', 'agentmoshpit.com'].every(words => said.includes(words))
    })
  )
  check('with a button to the download page', (await page.locator('.toast').getByRole('button', { name: 'Open the page' }).count()) === 1)
  await page.context().close()

  // Local voice states use simulated audio and downloads, with terminal delivery
  // observable through the existing demo bridge. No microphone is opened.
  async function voiceOffice(extra = '', width = 1280) {
    const p = await open(`demo=office&still${extra}`, width)
    await p.getByRole('button', { name: 'More', exact: true }).click()
    await p.getByRole('menuitem', { name: 'Voice input' }).click()
    check('voice starts off', !(await p.getByLabel('Enable voice input').isChecked()))
    check('new voice setup defaults to the recommended model on this computer', await p.getByRole('radio', { name: /^On this computer/ }).isChecked() && await p.getByRole('radio', { name: /^Nemotron Speech Streaming ?730 MB.*recommended/ }).isChecked())
    await p.getByLabel('Microphone', { exact: true }).selectOption('USB microphone')
    await p.getByLabel('Enable voice input').check()
    await p.getByRole('button', { name: 'Download Nemotron Speech Streaming', exact: true }).click()
    check('voice shows explicit model download progress', await until(async () => await p.getByRole('progressbar', { name: 'Model download' }).count() === 1))
    if (extra.includes('download-error')) return p
    await p.getByRole('button', { name: 'Remove Nemotron Speech Streaming', exact: true }).waitFor()
    await p.getByRole('button', { name: 'Close', exact: true }).click()
    await p.locator('[data-desk="demo-2"]').click()
    await p.locator('.pane[data-pane="demo-2"] .xterm').waitFor()
    return p
  }
  page = await voiceOffice()
  check('voice reserves no terminal shortcut while off by default', await page.getByRole('button', { name: 'Start voice input' }).isEnabled())
  // The microphone's own menu: which one is in use, and the way to another, without opening setup.
  await page.getByRole('button', { name: 'Microphone and speech engine' }).click()
  check('the arrow beside the microphone lists the microphones, the one in use ticked', await until(async () => await page.getByRole('menuitemradio', { name: 'USB microphone' }).getAttribute('aria-checked') === 'true') && await page.getByRole('menuitemradio', { name: /^System default/ }).getAttribute('aria-checked') === 'false')
  check('with one engine ready there is no engine to choose', await page.getByRole('menu', { name: 'Voice input' }).getByText('Speech engine').count() === 0)
  await page.keyboard.press('Escape')
  check('Escape closes it and nothing is recording', await page.getByRole('menu', { name: 'Voice input' }).count() === 0 && await page.getByRole('button', { name: 'Stop recording' }).count() === 0)
  const atRest = await page.locator('.bar .new').boundingBox()
  await page.locator('.pane[data-pane="demo-2"] .xterm-helper-textarea').focus()
  await page.getByRole('button', { name: 'Start voice input' }).click()
  check('the microphone does not take the keyboard from the terminal', await page.locator('.pane[data-pane="demo-2"] .xterm-helper-textarea').evaluate(el => el === document.activeElement))
  // The bar is in the terminal the words go into, so it need not name it to the eye; it is named aloud.
  check('voice visibly listens in blue, in the terminal it types into', await until(async () => ((await page.locator('.pane[data-pane="demo-2"] .voice-status .state').textContent()) ?? '').includes('Listening to Refactor auth middleware')) && await page.locator('.voice-status .state').evaluate(el => getComputedStyle(el).color === 'rgb(130, 170, 255)'))
  check('voice shows the words as they are spoken', await until(async () => (await page.locator('.voice-status .words').innerText()).includes('Fix the checkout')))
  check('voice uses the chosen microphone and displays incoming audio', await page.getByRole('meter', { name: 'Microphone input level' }).getAttribute('title') === 'USB microphone' && await page.getByRole('meter', { name: 'Microphone input level' }).getAttribute('aria-valuenow') === '36')
  const bar = await page.locator('.bar .new').boundingBox()
  check('the bar does not move when a recording starts', bar !== null && JSON.stringify(bar) === JSON.stringify(atRest))
  await page.locator('[data-desk="demo-6"]').click()
  check('focus changes mark only the selected pane as read while keeping both voice panes visible', await until(async () => await page.evaluate(() => JSON.stringify(window.__demo.watched) === '["demo-6"]' && ['demo-2', 'demo-6'].every(id => window.__demo.visible.includes(id)))))
  await page.getByRole('button', { name: 'Stop and insert' }).click()
  check('voice shows it is typing the last words', await until(async () => (await page.locator('.voice-status').innerText()).includes('Typing the last words')))
  check('voice inserts into the original session without Enter after focus changes', await until(async () => await page.evaluate(() => window.__demo.typed.some(t => t.agent === 'demo-2' && t.data === 'Fix the checkout total'))))
  check('voice never submits or leaves controls in its text', await page.evaluate(() => window.__demo.typed.every(t => !/[\r\n\x00-\x1f\x7f]/.test(t.data))))
  check('the words are in the terminal, so no notice is left over it', await until(async () => await page.getByRole('button', { name: 'Start voice input' }).count() === 1) && await page.locator('.voice-status').count() === 0)
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.getByRole('button', { name: 'Cancel voice input' }).click()
  const cancelled = await page.evaluate(() => window.__demo.typed.length)
  await sleep(800)
  check('cancelling voice inserts nothing', await page.evaluate(() => window.__demo.typed.length) === cancelled)
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.getByRole('button', { name: 'Stop and insert' }).click()
  // The recording is its program's, not its pane's: put away while the last words come, they still go in.
  await page.keyboard.press('Control+Shift+W')
  check('putting its pane away while the last words come still types them into its program', await until(async () => await page.evaluate(n => window.__demo.typed.length > n && window.__demo.typed.slice(n).every(t => t.agent === 'demo-6'), cancelled)))
  check('and says where they went, with the way to it', await until(async () => ((await page.locator('.voice-status').textContent()) ?? '').includes('Typed into Bump dependencies')) && await page.locator('.voice-status .where').count() === 1)
  await page.context().close()

  // On a Mac a Control-click is the system's right-click: it opens the menu and does nothing else. Cmd asks for more.
  page = await open('demo=office&still', 1280, 800, 'MacIntel')
  await desk(page, 2).click()
  // As the system sends it: the press, the menu, then the click as well.
  const onDesk = await desk(page, 5).boundingBox()
  await page.keyboard.down('Control')
  await page.mouse.move(onDesk.x + 20, onDesk.y + 20)
  await page.mouse.down()
  await desk(page, 5).dispatchEvent('contextmenu', { bubbles: true, cancelable: true, clientX: onDesk.x + 20, clientY: onDesk.y + 20 })
  await page.mouse.up()
  await page.keyboard.up('Control')
  check('on a Mac, a Control-click on a desk opens its menu and no terminal', (await page.locator('.menu-sheet').count()) === 1 && (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Refactor auth middleware')
  await page.keyboard.press('Escape')
  await desk(page, 5).click({ modifiers: ['Meta'] })
  check('and Cmd and a click shows it in place of the terminal in use', await until(async () => (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Docs pass for the API'))
  await page.keyboard.press('Control+Shift+Slash')
  check('the keys list names Cmd for the clicks there', (await page.locator('.panel').innerText()).includes('Cmd and a click') && !(await page.locator('.panel').innerText()).includes('Ctrl and a click'))
  await page.context().close()

  for (const [scene, message] of [['mic-error', 'microphone could not start'], ['silence', 'No clear speech'], ['short', 'too short'], ['inference-error', 'speech engine stopped']]) {
    page = await voiceOffice(`&voice=${scene}`)
    await page.getByRole('button', { name: 'Start voice input' }).click()
    if (scene !== 'mic-error') await page.getByRole('button', { name: 'Stop and insert' }).click()
    check(`voice ${scene} is helpful and inserts nothing`, await until(async () => ((await page.locator('.voice-status').textContent()) ?? '').includes(message)) && await page.evaluate(() => window.__demo.typed.length === 0))
    await page.context().close()
  }
  page = await voiceOffice('&voice=download-error')
  check('a failed model validation is visible and remains unavailable', await until(async () => (await page.getByRole('alert').textContent())?.includes('SHA-256')) && await page.getByRole('button', { name: 'Download Nemotron Speech Streaming', exact: true }).isEnabled())
  await page.context().close()

  page = await voiceOffice('', 420)
  await page.keyboard.press('Control+Shift+Space')
  check('configured shortcut starts voice from the terminal', await until(async () => await page.getByRole('button', { name: 'Stop and insert' }).count() === 1))
  const shots = path.join(root, '.impeccable', 'review')
  await (await import('node:fs/promises')).mkdir(shots, { recursive: true })
  await page.screenshot({ path: path.join(shots, 'voice-narrow.png') })
  check('voice fits a narrow window', await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth))
  await page.getByRole('button', { name: 'Cancel voice input' }).click()
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: 'Voice input' }).click()
  await page.screenshot({ path: path.join(shots, 'voice-panel-narrow.png') })
  await page.getByLabel('Shortcut', { exact: true }).selectOption('none')
  await page.getByLabel('Enable voice input').uncheck()
  check('disabling voice keeps a setup control and starts no capture', await until(async () => await page.getByRole('button', { name: 'Set up voice input' }).count() === 1) && await page.getByRole('button', { name: 'Stop recording' }).count() === 0)
  await page.getByRole('button', { name: 'Remove Nemotron Speech Streaming', exact: true }).click()
  check('downloaded models can be removed', await until(async () => await page.getByRole('button', { name: 'Download Nemotron Speech Streaming', exact: true }).count() === 1))
  await page.context().close()

  page = await open('demo=office&still&voice=inputs-error', 420)
  await page.getByRole('button', { name: 'Set up voice input' }).click()
  check('voice is discoverable without enabling it or recording', await page.getByRole('heading', { name: 'Voice input', exact: true }).count() === 1 && !(await page.getByLabel('Enable voice input').isChecked()))
  check('microphone discovery failure gives an inline refresh action', await until(async () => (await page.getByRole('alert').innerText()).includes('Reconnect')) && await page.getByRole('button', { name: 'Refresh microphones' }).isEnabled())
  await page.getByRole('button', { name: 'Refresh microphones' }).click()
  check('microphone refresh failure leaves setup usable', await page.getByLabel('Microphone', { exact: true }).isEnabled())
  await page.context().close()

  page = await voiceOffice()
  await page.getByRole('button', { name: 'Start voice input' }).click()
  await page.screenshot({ path: path.join(shots, 'voice-desktop.png') })
  await page.getByRole('button', { name: 'Cancel voice input' }).click()
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: 'Voice input' }).click()
  await page.screenshot({ path: path.join(shots, 'voice-panel-desktop.png') })
  await page.context().close()

  // ── review fixes: visible recovery and reliable attention ──
  for (const [width, height] of [[1024, 700], [340, 420]]) {
    page = await open('demo=office&still', width, height)
    await page.getByRole('button', { name: 'New agent', exact: true }).click()
    const folder = page.locator('#new-cwd')
    check(`the folder is filled in with where they most likely work at ${width}`, (await folder.inputValue()) === 'C:/code/shop')
    await folder.fill('')
    await page.getByRole('button', { name: 'Start agent', exact: true }).click()
    const fieldError = page.locator('#new-cwd-error')
    check(`missing Folder receives focus at ${width}`, await folder.evaluate(el => document.activeElement === el))
    check(`Folder error is linked and visible at ${width}`, await folder.getAttribute('aria-invalid') === 'true' && await folder.getAttribute('aria-describedby') === 'new-cwd-error' && await fieldError.evaluate(el => { const r = el.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight }))
    await folder.fill('C:/code/shop')
    check(`correcting Folder clears its error at ${width}`, await fieldError.count() === 0)
    await page.locator('details.options > summary').click()
    await page.locator('#new-title').focus()
    check(`focused Name is clear of the actions at ${width}`, await page.locator('#new-title').evaluate(el => el.getBoundingClientRect().bottom <= document.querySelector('.panel footer').getBoundingClientRect().top))
    await page.context().close()
  }

  page = await open('demo=office&still', 640, 700)
  await page.getByRole('button', { name: 'Open their terminal', exact: true }).click()
  await page.getByRole('button', { name: 'New agent', exact: true }).click()
  check('covering a waiting terminal does not remove its attention cue', await page.locator('.bar .attention').count() === 1)
  check('covered terminals are not reported as watched', await until(async () => (await page.evaluate(() => window.__demo.watched.length)) === 0))
  check('a recording cannot be started into a covered terminal', await page.evaluate(() => window.__demo.visible.length === 0))
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: /Show them/ }).click()
  await page.getByRole('menuitem', { name: /in trouble/ }).click()
  check('compact keyboard-accessible status control opens the trouble terminal', await until(async () => (await page.locator('.pane[data-pane="demo-7"]').count()) === 1))
  await page.context().close()

  page = await open('demo=office&still&problem')
  await desk(page, 2).click()
  check('desk navigation preserves the unresolved startup warning', await page.locator('.problem-strip').count() === 1)
  await page.locator('.problem-strip').getByRole('button', { name: 'Dismiss', exact: true }).click()
  check('startup warning can be explicitly dismissed', await page.locator('.problem-strip').count() === 0)
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitem', { name: /Configuration warning/ }).click()
  check('dismissed warning remains recoverable with its file action', await page.locator('.problem-strip').getByRole('button', { name: 'Open harnesses.json', exact: true }).count() === 1)
  await page.context().close()

  page = await open('demo=calm&still')
  await desk(page, 3).focus()
  await page.keyboard.press('Delete')
  await desk(page, 4).focus()
  await page.keyboard.press('Delete')
  const undoAll = page.getByRole('button', { name: 'Undo all', exact: true })
  check('consecutive removals retain one explicit batch Undo', await undoAll.count() === 1)
  await undoAll.focus()
  await sleep(8200)
  check('Undo does not expire while its action holds keyboard focus', await undoAll.count() === 1)
  await undoAll.click()
  check('batch Undo restores both removed desks', await until(async () => (await desk(page, 3).count()) === 1 && (await desk(page, 4).count()) === 1))
  await page.context().close()

  page = await open('demo=crowd&still', 1280, 800)
  const roomTops = await page.locator('.floor .room').evaluateAll(rooms => rooms.slice(0, 2).map(room => Math.round(room.getBoundingClientRect().top)))
  check('two crowd rooms share a row at 1280', roomTops.length === 2 && roomTops[0] === roomTops[1])
  await page.context().close()

  page = await open('demo=office&still', 1024, 700)
  await desk(page, 2).click()
  await desk(page, 5).click()
  check('two laptop panes start with a compact floor', await page.locator('.floor').evaluate(el => el.getBoundingClientRect().width < 300))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitemcheckbox', { name: /Compact floor/ }).click()
  check('a visible control expands the floor when requested', await page.locator('.floor').evaluate(el => el.getBoundingClientRect().width >= 400))
  await page.context().close()

  page = await open('demo=crowd&still&update', 340, 420)
  check('minimum-size attention retains an identity slot', await page.locator('.band .who').evaluate(el => el.getBoundingClientRect().width > 80))
  check('minimum-size attention stays within the viewport', await page.locator('.band').evaluate(el => el.scrollWidth <= el.clientWidth))
  check('minimum-size crowd gives each nameplate the remaining row', await page.locator('.floor .plate').first().evaluate(el => el.getBoundingClientRect().width > 120))
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.keyboard.press('End')
  check('More menu scrolls internally without moving the app', await page.locator('.sheet').evaluate(el => { const r = el.getBoundingClientRect(); return scrollY === 0 && r.top >= 0 && r.bottom <= innerHeight }))
  await page.context().close()

  page = await open('demo=office&still', 1024, 700)
  await desk(page, 6).click({ button: 'right' })
  await page.getByRole('menuitem', { name: /Stop their program/ }).click()
  await desk(page, 6).click({ button: 'right' })
  check('latest-conversation menu hint is readable in full', await page.getByRole('menuitem', { name: /Continue latest/ }).locator('.hint').evaluate(el => el.scrollHeight <= el.clientHeight))
  await page.context().close()

  // ── the smart office: attention, plain clicks and wide screens ──
  page = await open('demo=crowd&still')
  await page.keyboard.press('Control+Shift+KeyA')
  check('Ctrl+Shift+A opens whoever has waited longest', await until(async () => ((await page.locator('.pane h2').first().textContent()) ?? '') === 'Task 13 in infra'))
  check('and the bar says who else is waiting', ((await page.locator('.bar .attention .more').textContent()) ?? '') === '+1')
  await page.locator('.pane .term-host').first().click()
  await page.keyboard.press('Control+Shift+KeyA')
  check('again, from inside a terminal, the next one', await until(async () => (await page.locator('.pane h2', { hasText: 'Task 5 in mobile' }).count()) === 1))
  await page.context().close()

  page = await open('demo=crowd&still')
  await page.locator('.band .queue button', { hasText: 'Task 5 in mobile' }).click()
  check('an "also waiting" name opens their terminal in one click', await until(async () => (await page.locator('.pane h2', { hasText: 'Task 5 in mobile' }).count()) === 1))
  await page.context().close()

  page = await open('demo=office&still')
  await page.getByRole('button', { name: 'Open their terminal' }).click()
  await until(async () => (await panes(page)) === 1)
  await desk(page, 2).click(HELD)
  check('a desk swapped in never takes the place of someone who needs you', await until(async () => (await panes(page)) === 2 && (await page.locator('.pane h2', { hasText: 'Fix the checkout total' }).count()) === 1))
  // Nor does a click on their own desk put them away: it goes to their terminal, where the answer is wanted.
  await desk(page, 1).click()
  check('a click on someone who needs you goes to their terminal and does not put it away', await until(async () => ((await page.locator('.pane.typing h2').textContent()) ?? '') === 'Fix the checkout total') && (await panes(page)) === 2)
  await page.context().close()

  page = await open('demo=office&still', 2400, 1000)
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 1)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 5).click()
  check('a click from the floor brings back the terminals that were open, with the new one beside them', await until(async () => (await panes(page)) === 2))
  await page.context().close()

  // A narrow window shows the floor or the terminals, never both: a desk there is somewhere to go.
  page = await open('demo=office&still', 420, 760)
  await desk(page, 2).click()
  await until(async () => (await panes(page)) === 1)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 5).click()
  check('in a narrow window a click goes to their terminal, one at a time', await until(async () => (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Docs pass for the API'))
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 6).click(HELD)
  check('and the key held opens one beside it', await until(async () => (await panes(page)) === 2))
  await page.context().close()

  page = await open('demo=office&still')
  await page.getByRole('button', { name: 'New agent', exact: true }).click()
  check('New agent folds its options into one line until asked', !(await page.locator('#new-model').isVisible()) && (await page.locator('details.options > summary').textContent())?.includes('More options'))
  await page.context().close()

  // ── signing in: who is signed in to each program, switching, and a desk whose program is signed out ──
  // ── what agents sent each other, and how each is told ──
  {
    const mail = await open('demo=office&still')
    await mail.getByRole('button', { name: 'More', exact: true }).click()
    await mail.getByRole('menuitem', { name: 'Agent messages' }).click()
    const panel = mail.locator('aside.panel[aria-label="Agent messages"]')
    await panel.getByRole('heading', { name: 'What was sent' }).waitFor()
    const told = name => panel.getByRole('checkbox', { name: new RegExp(`^${name}`) })
    check('agents are told at a step of their work and woken when idle unless that is turned off; pushes are off until turned on', (await told('At the next step of its work').isChecked()) && (await told('Wake it when it is idle').isChecked()) && !(await told('Push it in as it arrives').isChecked()))
    check('pushes are said to be a preview that asks for confirmation each start', ((await panel.locator('label', { hasText: 'Push it in as it arrives' }).textContent()) ?? '').replace(/\s+/g, ' ').includes('research preview') && ((await panel.locator('label', { hasText: 'Push it in as it arrives' }).textContent()) ?? '').includes('asks you to confirm in its terminal each time it starts'))
    await told('Push it in as it arrives').check()
    check('turning pushes on is passed to the core, with the others as they were', await until(async () => JSON.stringify(await mail.evaluate(() => window.__demo.delivery.at(-1))) === JSON.stringify({ hooks: true, wake: true, channels: true })))
    await told('At the next step of its work').uncheck()
    check('without the hooks there is nothing to wake with, and the choice says so by being unavailable', (await told('Wake it when it is idle').isDisabled()) && !(await told('Wake it when it is idle').isChecked()) && (await mail.evaluate(() => window.__demo.delivery.at(-1).hooks)) === false)
    await told('At the next step of its work').check()
    check('the choice to wake is remembered when the hooks come back', (await told('Wake it when it is idle').isChecked()) && !(await told('Wake it when it is idle').isDisabled()))
    const reached = name => panel.locator('.desks li', { has: mail.locator('.name', { hasText: new RegExp(`^${name}$`) }) })
    check('each agent says how it is reached, by what has been seen and not by what was hoped', ((await reached('Fix the checkout total').locator('.how').textContent()) ?? '') === 'Connected: told at a step of its work, and woken when idle.' && ((await reached('Refactor auth middleware').locator('.how').textContent()) ?? '').includes('next call to the office, or when it checks its inbox'))
    check('an agent that is not running keeps what is sent to it, and says how much is unread', ((await reached('Flaky test hunt').locator('.how').textContent()) ?? '').startsWith('Not running. What is sent is kept') && ((await reached('Flaky test hunt').locator('.tag').textContent()) ?? '').includes('1 unread'))
    check('only the programs that can take messages are listed', (await panel.locator('.desks li').count()) === 5 && (await reached('Bump dependencies').count()) === 0)
    const sent = text => panel.locator('.items li', { hasText: text })
    check('a message that was acknowledged says so, with who sent it to whom', /^Acknowledged .+\.$/.test((await sent('session check now runs').locator('.how').textContent()) ?? '') && ((await sent('session check now runs').locator('.name').textContent()) ?? '').replace(/\s+/g, ' ').includes('Refactor auth middleware → to Fix the checkout total'))
    check('a message only mentioned is not said to have been read', /^Said with a tool's answer .+\. Not read yet\.$/.test((await sent('orders routes are typed').locator('.how').textContent()) ?? ''))
    check('a task says its own state apart from whether it arrived', ((await sent('Run the checkout tests').locator('.tag').textContent()) ?? '') === 'task · not taken up yet' && ((await sent('Run the checkout tests').locator('.how').textContent()) ?? '') === 'Saved. Not told yet.')
    check('the panel fits its column', await mail.evaluate(() => document.documentElement.scrollWidth <= innerWidth) && await panel.locator('.body').evaluate(el => el.scrollWidth <= el.clientWidth))
    await mail.keyboard.press('Escape')
    check('Escape closes it', (await panel.count()) === 0)
    await mail.close()

    for (const [scene, says] of [['off', "The office's connection for agents is off"], ['none', 'Nothing yet. Ask an agent to send another one a message']]) {
      const quiet = await open(`demo=office&still&mail=${scene}`, 420, 800)
      await quiet.getByRole('button', { name: 'More', exact: true }).click()
      await quiet.getByRole('menuitem', { name: 'Agent messages' }).click()
      check(`with ${scene === 'off' ? 'the connection off' : 'nothing sent'} the panel says so, in a narrow window too`, await until(async () => ((await quiet.locator('aside.panel[aria-label="Agent messages"]').textContent()) ?? '').includes(says)) && await until(() => quiet.evaluate(() => document.documentElement.scrollWidth <= innerWidth)))
      check(`and ${scene === 'off' ? 'offers no settings that could do nothing' : 'still offers how agents are told'}`, (await quiet.locator('aside.panel[aria-label="Agent messages"] input[type="checkbox"]').count()) === (scene === 'off' ? 0 : 3))
      await quiet.close()
    }
  }

  // ── a program that needs Node.js, on a computer without it ──
  {
    const bare = await open('demo=office&still&nonode')
    const looked = () => bare.evaluate(() => window.__demo.looked)
    await bare.getByRole('button', { name: 'New agent' }).first().click()
    check('the programs are looked for again when New agent opens', (await looked()) === 1)
    await bare.getByRole('button', { name: /to install$/ }).click()
    await bare.locator('.pick label', { hasText: 'OpenCode' }).click()
    const hint = bare.locator('aside.panel .hint', { hasText: 'Node.js' })
    check('a program npm installs says Node.js is missing before anything is pressed', /was not found on this computer\..*nothing to restart/.test(((await hint.textContent()) ?? '').replace(/\s+/g, ' ')))
    check('with a way to get it', (await bare.getByRole('button', { name: 'Get Node.js', exact: true }).count()) === 1 && (await bare.locator('aside.panel code.line').count()) === 0)
    await bare.getByRole('button', { name: 'Install OpenCode and start' }).click()
    check('pressing install anyway says the same, and that no restart is needed', await until(async () => ((await bare.locator('aside.panel footer .problem').textContent().catch(() => '')) ?? '').includes('the office does not need restarting')))
    check('and nothing was started', (await bare.evaluate(() => window.__demo.specs.length)) === 0 && (await bare.locator('.pane').count()) === 0)
    await bare.keyboard.press('Escape')

    await bare.getByRole('button', { name: 'More', exact: true }).click()
    await bare.getByRole('menuitem', { name: /Agent programs/ }).click()
    check('Agent programs says so once, above the programs that are not here, with the way to get it', (await bare.locator('aside.panel .needs').count()) === 1 && ((await bare.locator('aside.panel .needs .note').textContent()) ?? '').replace(/\s+/g, ' ').includes('need Node.js, which was not found') && (await bare.getByRole('button', { name: 'get Node.js', exact: true }).count()) === 1)
    check('the programs that are here are not under it', (await bare.locator('aside.panel .program-group', { has: bare.locator('.needs') }).locator('li:not(.absent)').count()) === 0)
    await bare.keyboard.press('Escape')

    // Node.js is installed while the office is open: the next look finds it, with no restart.
    await bare.evaluate(() => window.__demo.findNode())
    await bare.getByRole('button', { name: 'New agent' }).first().click()
    check('once Node.js is there, the program is offered to install as usual, without restarting', await until(async () => (await bare.locator('aside.panel code.line').count()) === 1 && (await bare.getByRole('button', { name: 'Get Node.js', exact: true }).count()) === 0))
    await bare.getByRole('button', { name: 'Install OpenCode and start' }).click()
    check('and installing it starts', await until(async () => (await bare.locator('.pane').count()) === 1))
    await bare.close()
  }

  {
    const programs = async p => {
      await p.getByRole('button', { name: 'More', exact: true }).click()
      await p.getByRole('menuitem', { name: /Agent programs/ }).click()
      await p.locator('aside.panel .program-group').first().waitFor()
    }
    const row = (p, name) => p.locator('aside.panel li', { has: p.locator('.name', { hasText: new RegExp(`^${name}$`) }) })

    let signs = await open('demo=office&still')
    await programs(signs)
    check('the programs are asked who is signed in when their panel opens', (await signs.evaluate(() => window.__demo.asked)) >= 1)
    check('each says who, in its own words', ((await row(signs, 'Claude Code').locator('.account').textContent()) ?? '') === 'Signed in as you@example.com · Max' && ((await row(signs, 'Codex').locator('.account').textContent()) ?? '') === 'Logged in using ChatGPT')
    check('with the command that signs in, and a way to sign in as someone else', ((await row(signs, 'Claude Code').locator('.do code').last().textContent()) ?? '') === 'claude auth login' && (await row(signs, 'Claude Code').getByRole('button', { name: 'switch account' }).count()) === 1)
    check('a program the office cannot sign in to offers nothing of the kind', (await row(signs, 'Hermes').locator('.account').count()) === 0 && (await row(signs, 'Hermes').getByRole('button', { name: /sign in|switch account/ }).count()) === 0)
    check('with desks of it running, it is said that they keep their sign-in until restarted', ((await row(signs, 'Claude Code').locator('.note').last().textContent()) ?? '').includes('sign-in') && ((await row(signs, 'Claude Code').locator('.note').last().textContent()) ?? '').includes('restarted'))
    check('the panel still fits its own buttons, with the sign-in rows in it', await signs.locator('aside.panel .do .button').evaluateAll(buttons => buttons.length > 0 && buttons.every(b => b.getBoundingClientRect().right <= b.closest('aside').getBoundingClientRect().right - 8 && b.scrollWidth <= b.clientWidth + 1)))
    await row(signs, 'Claude Code').getByRole('button', { name: 'switch account' }).click()
    check('signing in runs the program’s own command in a pane, where it can be watched', await until(async () => (await signs.locator('.pane[aria-label="Claude Code signed in"]').count()) === 1 && ((await signs.locator('.pane[data-pane="job-signin-claude"] .xterm-accessibility-tree').innerText()) ?? '').includes('claude auth login')), await signs.locator('.pane').evaluateAll(ps => ps.map(p => p.getAttribute('aria-label')).join(' | ')))
    check('and the pane says what it is', ((await signs.locator('.pane[data-pane="job-signin-claude"] .meta').textContent()) ?? '') === 'sign-in')
    await programs(signs)
    check('after which the panel says who is signed in now', await until(async () => ((await row(signs, 'Claude Code').locator('.account').textContent()) ?? '') === 'Signed in as other@example.com · Max'))
    await signs.context().close()

    // Nobody signed in: said where the programs are, on a desk of that program, and when one is about to start.
    signs = await open('demo=office&still&signedout=claude')
    await programs(signs)
    check('a program nobody is signed in to says so, and offers to sign in', ((await row(signs, 'Claude Code').locator('.account').textContent()) ?? '') === 'Signed out' && (await row(signs, 'Claude Code').getByRole('button', { name: 'sign in', exact: true }).count()) === 1 && (await row(signs, 'Codex').getByRole('button', { name: 'switch account' }).count()) === 1)
    await signs.keyboard.press('Escape')
    await desk(signs, 3).click()
    await until(async () => (await panes(signs)) === 1)
    const foot = signs.locator('.pane[data-pane="demo-3"] .ended')
    check('a stopped desk of that program says it is signed out, not that it stopped', ((await foot.locator('span').first().textContent()) ?? '') === 'Claude Code is signed out.' && (await foot.getByRole('button', { name: 'Sign in' }).count()) === 1 && (await foot.getByRole('button', { name: 'Carry on' }).count()) === 0, await foot.textContent())
    await foot.getByRole('button', { name: 'Sign in' }).click()
    check('Sign in starts its program there, which asks to be signed in to', await until(async () => (await signs.locator('.pane[data-pane="demo-3"] .ended').count()) === 0))
    await signs.getByRole('button', { name: 'New agent', exact: true }).click()
    check('New agent says so before one is started', await until(async () => ((await signs.locator('aside.panel .hint').first().textContent()) ?? '').includes('Nobody is signed in to Claude Code')))
    await signs.context().close()

    // A sign-in waiting on the browser: its pane can be put away and brought back, or the sign-in stopped.
    signs = await open('demo=office&signin=waits')
    await programs(signs)
    await row(signs, 'Codex').getByRole('button', { name: 'switch account' }).click()
    check('while it waits, its pane says it is signing in', await until(async () => (await signs.locator('.pane[aria-label="signing in to Codex"]').count()) === 1))
    await signs.locator('.pane[data-pane="job-signin-codex"]').getByRole('button', { name: 'Put this terminal away' }).click()
    await until(async () => (await signs.locator('.pane[data-pane="job-signin-codex"]').count()) === 0)
    await programs(signs)
    check('put away, the sign-in is still there to show or to stop', (await row(signs, 'Codex').getByRole('button', { name: 'show' }).count()) === 1 && (await row(signs, 'Codex').getByRole('button', { name: 'stop' }).count()) === 1 && (await row(signs, 'Codex').getByRole('button', { name: 'switch account' }).count()) === 0)
    await row(signs, 'Codex').getByRole('button', { name: 'show' }).click()
    check('show brings its pane back', await until(async () => (await signs.locator('.pane[data-pane="job-signin-codex"]').count()) === 1))
    await signs.evaluate(() => window.__demo.finishSignIn())
    check('and when the browser comes back, the pane says it is done', await until(async () => (await signs.locator('.pane[aria-label="Codex signed in"]').count()) === 1))
    await programs(signs)
    await row(signs, 'Codex').getByRole('button', { name: 'switch account' }).click()
    await until(async () => (await signs.locator('.pane[aria-label="signing in to Codex"]').count()) === 1)
    await programs(signs)
    await row(signs, 'Codex').getByRole('button', { name: 'stop' }).click()
    check('stop ends a sign-in that is waiting, and puts its pane away', await until(async () => (await signs.locator('.pane[data-pane="job-signin-codex"]').count()) === 0 && (await row(signs, 'Codex').getByRole('button', { name: 'switch account' }).count()) === 1))
    check('leaving whoever was signed in as they were', ((await row(signs, 'Codex').locator('.account').textContent()) ?? '') === 'Logged in using ChatGPT')
    await signs.context().close()
  }

  // ── arranging the terminals: a pane put on a side of another, a whole side of the room, keys, shapes ──
  {
    const room = await open('demo=office&still')
    await desk(room, 2).click()
    await desk(room, 5).click()
    await desk(room, 6).click()
    await until(async () => (await panes(room)) === 3)
    await sleep(300)
    // Every pane as the page first made it, and a count of any taken out of the page since.
    await room.evaluate(() => {
      window.__made = new Map([...document.querySelectorAll('.pane')].map(p => [p.dataset.pane, p]))
      window.__taken = 0
      new MutationObserver(changes => changes.forEach(c => c.removedNodes.forEach(n => n instanceof HTMLElement && n.classList.contains('pane') && window.__taken++))).observe(document.querySelector('.panes'), { childList: true })
    })
    const places = async () => Object.fromEntries((await laidOut(room)).map(p => [p.id, [p.x, p.y, p.w, p.h]]))
    const order = async () => (await laidOut(room)).map(p => p.id.replace('demo-', '')).join(' ')
    const box = id => room.locator(`.pane[data-pane="demo-${id}"]`).boundingBox()
    const near = (a, b) => a.every((n, i) => Math.abs(n - b[i]) <= 2)
    /** Take a pane by its header and hold it at a place in another pane, given as shares of that pane's box. */
    const hold = async (id, over, fx, fy) => {
      const head = await room.locator(`.pane[data-pane="demo-${id}"] header .status`).boundingBox()
      const target = typeof over === 'number' ? await box(over) : await room.locator('.panes').boundingBox()
      await room.mouse.move(head.x + 4, head.y + 4)
      await room.mouse.down()
      await room.mouse.move(target.x + target.width * fx, target.y + target.height * fy, { steps: 12 })
      await sleep(320)
    }
    const landing = async () => {
      if ((await room.locator('.panes .landing').count()) === 0) return null
      const ring = await room.locator('.panes .landing').boundingBox()
      return ring ? [ring.x, ring.y, ring.width, ring.height].map(Math.round) : null
    }
    const said = () => room.locator('.panes > .aloud').textContent()

    // Two above one. The one below, held at the right of the top right pane: it would stand third in that row.
    const before = await places()
    await hold(6, 5, 0.85, 0.5)
    const ring = await landing()
    check('a pane held over the side of another shows where it would land', ring !== null, JSON.stringify(ring))
    check('and nothing moves while it is held', JSON.stringify(await places()) === JSON.stringify(before))
    await room.mouse.up()
    check('let go, it stands on that side: three side by side', await until(async () => (await order()) === '2 5 6' && new Set((await laidOut(room)).map(p => p.y)).size === 1), JSON.stringify(await places()))
    check('exactly where the ring was', ring !== null && near((await places())['demo-6'], ring), `${JSON.stringify((await places())['demo-6'])} against ${JSON.stringify(ring)}`)
    check('and it is said aloud', ((await said()) ?? '') === 'Bump dependencies is now right of Docs pass for the API.', await said())
    check('with the keyboard in the terminal that moved', await until(() => room.evaluate(() => document.activeElement?.closest('.pane')?.dataset.pane === 'demo-6')))

    // Escape puts a held pane back, and the program in the terminal never hears it.
    const three = await places()
    await hold(6, 2, 0.5, 0.8)
    check('the ring follows the pane to wherever it is held', (await landing()) !== null)
    await room.keyboard.press('Escape')
    check('Escape lets it go where it was', (await landing()) === null)
    await room.mouse.up()
    await sleep(200)
    check('and nothing has moved', JSON.stringify(await places()) === JSON.stringify(three), JSON.stringify(await places()))
    check('and the program was not sent the Escape', await room.evaluate(() => !window.__demo.typed.some(t => t.data.includes('\x1b'))))

    // Below another: the two share the room that one had.
    await hold(6, 5, 0.5, 0.82)
    await room.mouse.up()
    check(
      'held at the foot of a pane, it goes below it, in the room that pane had',
      await until(async () => {
        const p = await places()
        return (await order()) === '2 5 6' && near([p['demo-6'][0], p['demo-6'][2]], [p['demo-5'][0], p['demo-5'][2]]) && p['demo-6'][1] > p['demo-5'][1] && p['demo-2'][3] > p['demo-5'][3] * 1.8
      }),
      JSON.stringify(await places())
    )

    // Where it would change nothing, nothing is offered.
    await hold(6, 5, 0.5, 0.82)
    check('held where it already stands, no ring is shown', (await landing()) === null, JSON.stringify(await landing()))
    await room.mouse.up()

    // The edge of the room: the whole side.
    const whole = await room.locator('.panes').boundingBox()
    await hold(2, 'room', 0.75, 0.99)
    const strip = await landing()
    check('held at the edge of the room, it would have that whole side', strip !== null && near([strip[0], strip[2]], [Math.round(whole.x), Math.round(whole.width)]), JSON.stringify(strip))
    await room.mouse.up()
    check('and takes it: one above another, the last across the whole foot', await until(async () => (await order()) === '5 6 2' && new Set((await laidOut(room)).map(p => p.x)).size === 1), JSON.stringify(await places()))
    check('said aloud as the side it has', ((await said()) ?? '') === 'Refactor auth middleware has the bottom of the room.', await said())

    // A header: the two change places.
    const stacked = await places()
    const head5 = await room.locator('.pane[data-pane="demo-5"] header').boundingBox()
    const from2 = await room.locator('.pane[data-pane="demo-2"] header .status').boundingBox()
    await room.mouse.move(from2.x + 4, from2.y + 4)
    await room.mouse.down()
    await room.mouse.move(head5.x + head5.width / 2, head5.y + head5.height / 2, { steps: 12 })
    await sleep(320)
    await room.mouse.up()
    check('let go on another pane’s header, the two change places', await until(async () => JSON.stringify((await places())['demo-2']) === JSON.stringify(stacked['demo-5']) && JSON.stringify((await places())['demo-5']) === JSON.stringify(stacked['demo-2'])), JSON.stringify(await places()))

    // The keys: Ctrl+Shift and an arrow to the pane that way; with Alt, the pane itself.
    await room.locator('.pane[data-pane="demo-6"] .term-host').click()
    const typingIn = () => room.evaluate(() => document.querySelector('.pane.typing')?.dataset.pane ?? '')
    await room.keyboard.press('Control+Shift+ArrowUp')
    check('Ctrl+Shift+Up takes the keyboard to the pane above', await until(async () => (await typingIn()) === 'demo-2'), await typingIn())
    await room.keyboard.press('Control+Shift+ArrowDown')
    await room.keyboard.press('Control+Shift+ArrowDown')
    check('and Down, twice, to the one at the foot', await until(async () => (await typingIn()) === 'demo-5'), await typingIn())
    await room.keyboard.press('Control+Shift+ArrowDown')
    check('at the edge of the room it stays where it is', (await typingIn()) === 'demo-5')
    await room.keyboard.press('Control+Shift+Alt+ArrowUp')
    check('Ctrl+Shift+Alt+Up moves that pane up a place', await until(async () => (await order()) === '2 5 6'), await order())
    await room.keyboard.type('k')
    check('and the keyboard goes with it', await until(() => room.evaluate(() => window.__demo.typed.at(-1)?.agent === 'demo-5' && window.__demo.typed.at(-1)?.data === 'k')), JSON.stringify(await room.evaluate(() => window.__demo.typed.slice(-2))))
    await room.keyboard.press('Control+Shift+Alt+ArrowLeft')
    check(
      'with nobody on that side, it takes that whole side of the room',
      await until(async () => {
        const p = (await places())['demo-5']
        return near([p[0], p[1], p[3]], [Math.round(whole.x), Math.round(whole.y), Math.round(whole.height)])
      }),
      JSON.stringify(await places())
    )
    check('none of those keys reached a program', await room.evaluate(() => !window.__demo.typed.some(t => t.data.includes('\x1b'))), JSON.stringify(await room.evaluate(() => window.__demo.typed)))

    // Arrange: every terminal in one shape, with a way back.
    const built = await places()
    await room.getByRole('button', { name: 'Arrange', exact: true }).click()
    check('Arrange offers each shape, an even share, and a step each way', (await room.getByRole('menuitem').evaluateAll(items => items.map(i => i.textContent.trim() || i.getAttribute('aria-label')).join('|'))) === 'In a grid|Side by side|Stacked|This one large, the others beside it|This one large, the others below it|Make them even|Move Docs pass for the API left|Move Docs pass for the API right|Move Docs pass for the API up|Move Docs pass for the API down', await room.getByRole('menuitem').evaluateAll(items => items.map(i => i.textContent.trim() || i.getAttribute('aria-label')).join('|')))
    await room.getByRole('menuitem', { name: 'Stacked' }).click()
    check('Stacked puts them one above another', await until(async () => new Set((await laidOut(room)).map(p => p.x)).size === 1 && new Set((await laidOut(room)).map(p => p.h)).size === 1), JSON.stringify(await places()))
    check('and says so, with a way back', ((await room.locator('.toast').textContent()) ?? '').includes('Arranged one above another.') && (await room.locator('.toast').getByRole('button', { name: 'Undo' }).count()) === 1, await room.locator('.toast').textContent())
    await room.locator('.toast').getByRole('button', { name: 'Undo' }).click()
    check('Undo puts the room back as it was built', await until(async () => JSON.stringify(await places()) === JSON.stringify(built)), JSON.stringify(await places()))
    const shape = async name => {
      await room.getByRole('button', { name: 'Arrange', exact: true }).click()
      await room.getByRole('menuitem', { name }).click()
      await sleep(250)
      return laidOut(room)
    }
    let shaped = await shape('Side by side')
    check('Side by side stands them in one row', new Set(shaped.map(p => p.y)).size === 1 && new Set(shaped.map(p => p.w)).size <= 2, JSON.stringify(shaped))
    shaped = await shape('In a grid')
    check('In a grid is two above one', shaped.filter(p => p.y === shaped[0].y).length === 2 && shaped.length === 3, JSON.stringify(shaped))
    shaped = await shape('This one large, the others beside it')
    const mine = await typingIn()
    check('This one large gives the pane the keyboard is in the height of the room and most of its width', shaped[0].id === mine && Math.abs(shaped[0].h - whole.height) <= 2 && shaped[0].w > shaped[1].w * 1.3 && shaped[1].x === shaped[2].x, JSON.stringify(shaped))
    check('and names who has it', ((await room.locator('.toast').textContent()) ?? '').includes('Docs pass for the API is the large one.'), await room.locator('.toast').textContent())
    shaped = await shape('This one large, the others below it')
    check('or the width of the room, with the others in a row below', shaped[0].id === mine && Math.abs(shaped[0].w - whole.width) <= 2 && shaped[1].y === shaped[2].y && shaped[0].h > shaped[1].h * 1.3, JSON.stringify(shaped))
    check('the keyboard is back in that terminal once the room is arranged', await until(() => room.evaluate(() => document.activeElement?.closest('.pane')?.dataset.pane === document.querySelector('.pane.typing')?.dataset.pane)))
    await room.getByRole('button', { name: 'Arrange', exact: true }).click()
    await room.getByRole('menuitem', { name: 'Move Docs pass for the API down' }).click()
    check('a step from Arrange moves the pane and leaves Arrange open for the next', await until(async () => (await laidOut(room)).at(-1).id === mine || (await laidOut(room))[0].id !== mine) && (await room.getByRole('menuitem', { name: 'Stacked' }).count()) === 1, JSON.stringify(await laidOut(room)))
    await room.keyboard.press('Escape')
    check('Escape closes Arrange', await until(async () => (await room.getByRole('menuitem', { name: 'Stacked' }).count()) === 0))

    check('through all of it, no terminal was taken out of the page or made again', await room.evaluate(() => window.__taken === 0 && [...document.querySelectorAll('.pane')].every(p => window.__made.get(p.dataset.pane) === p)), String(await room.evaluate(() => window.__taken)))

    // One pane given the room: the others are behind it, and nothing would be seen to move.
    await room.keyboard.press('Control+Shift+Enter')
    await until(async () => (await room.locator('.pane:not(.hidden)').count()) === 1)
    const zoomed = await order()
    await room.keyboard.press('Control+Shift+Alt+ArrowRight')
    await sleep(200)
    check('a pane given the room is not moved from behind it', (await order()) === zoomed && ((await room.locator('.toast').textContent()) ?? '').includes('Put the other terminals back'), await room.locator('.toast').textContent().catch(() => ''))
    await room.keyboard.press('Control+Shift+Enter')

    // With one terminal there is nothing to arrange; in a narrow window it is in the menu.
    await room.setViewportSize({ width: 700, height: 800 })
    await sleep(300)
    check('a narrow window has no Arrange in its bar', (await room.getByRole('button', { name: 'Arrange', exact: true }).count()) === 0)
    await room.getByRole('button', { name: 'More', exact: true }).click()
    check('its menu arranges instead', (await room.getByRole('menuitem', { name: 'Side by side' }).count()) === 1 && (await room.getByRole('menuitem', { name: 'Make them even' }).count()) === 1)
    await room.getByRole('menuitem', { name: 'Side by side' }).click()
    check('and does', await until(async () => new Set((await laidOut(room)).map(p => p.y)).size === 1), JSON.stringify(await places()))
    await room.setViewportSize({ width: 1280, height: 800 })
    await room.locator('.pane[data-pane="demo-6"]').getByRole('button', { name: 'Put this terminal away' }).click()
    await room.locator('.pane[data-pane="demo-5"]').getByRole('button', { name: 'Put this terminal away' }).click()
    check('with one terminal open there is no Arrange', await until(async () => (await panes(room)) === 1 && (await room.getByRole('button', { name: 'Arrange', exact: true }).count()) === 0))

    // In a form, Ctrl+Shift and an arrow is the field's own: the office does not take it, and
    // where it selects by the word it still does. (macOS selects by the word with Option.)
    await room.getByRole('button', { name: 'New agent', exact: true }).click()
    await room.locator('#new-task').fill('alpha beta')
    await room.evaluate(() => addEventListener('keydown', event => { if (event.key === 'ArrowLeft') window.__arrow = event }, { capture: true }))
    await room.keyboard.press('Control+Shift+ArrowLeft')
    const field = await room.evaluate(() => {
      const el = document.activeElement
      return { here: el?.id === 'new-task', taken: window.__arrow?.defaultPrevented, chosen: el?.value?.slice(el.selectionStart, el.selectionEnd) }
    })
    check('in a text field Ctrl+Shift+Left is left to the field', field.here && field.taken === false && (process.platform === 'darwin' || field.chosen === 'beta'), JSON.stringify(field))
    await room.context().close()

    // What an earlier office kept, rows of panes with their shares, is still read.
    const kept = await browser.newContext({ viewport: { width: 1280, height: 800 }, colorScheme: 'light' })
    await kept.addInitScript(() => {
      localStorage.setItem('moshpit.view', JSON.stringify({ floor: 264, floorChosen: true, open: true, layout: { rows: [{ size: 1, panes: [{ id: 'demo-2', size: 1.4 }, { id: 'demo-5', size: 0.6 }] }, { size: 2, panes: [{ id: 'demo-6', size: 1 }] }] } }))
    })
    const again = await kept.newPage()
    again.on('pageerror', error => errors.push(String(error)))
    await again.goto(`${base}?demo=office&still`)
    await again.waitForSelector('.pane')
    await until(async () => (await panes(again)) === 3)
    const old = await laidOut(again)
    check('the terminals an earlier office kept come back where they were', old.map(p => p.id).join(' ') === 'demo-2 demo-5 demo-6' && old[0].y === old[1].y && old[2].y > old[0].y, JSON.stringify(old))
    check('with the shares they were dragged to', Math.abs(old[0].w / old[1].w - 1.4 / 0.6) < 0.05 && Math.abs(old[2].h / old[0].h - 2) < 0.05, JSON.stringify(old))
    await kept.close()

    // The bar keeps its menu at every width, with one more button in it.
    for (const width of [1101, 901, 761, 621, 420, 340]) {
      const bar = await open('demo=office&still', 1280, 720)
      await bar.getByRole('button', { name: 'Open their terminal' }).click()
      await desk(bar, 2).click()
      await until(async () => (await panes(bar)) === 2)
      await bar.setViewportSize({ width, height: 720 })
      await sleep(300)
      const fits = await bar.evaluate(() => {
        const more = document.querySelector('.bar .more').getBoundingClientRect()
        const strip = document.querySelector('.bar').getBoundingClientRect()
        return { more: Math.round(more.right), bar: Math.round(strip.height), page: document.documentElement.scrollWidth, wide: innerWidth, arrange: document.querySelectorAll('.bar .arrange').length }
      })
      check(`at ${width} wide the bar still holds its menu, with two terminals open and someone waiting`, fits.more <= fits.wide && fits.bar <= 53 && fits.page <= fits.wide && fits.arrange === (width > 760 ? 1 : 0), JSON.stringify(fits))
      await bar.context().close()
    }
  }

  check('nothing went wrong on the page', errors.length === 0, errors.join(' | '))
} catch (error) {
  check('the checks ran to the end', false, error?.stack ?? error)
} finally {
  await browser.close()
  server.kill()
}

for (const c of checks) console.log(`${c.ok ? 'ok  ' : 'FAIL'} ${c.name}${c.ok || !c.detail ? '' : `  (${c.detail})`}`)
const failed = checks.filter(c => !c.ok).length
console.log(failed === 0 ? `\nall ${checks.length} passed` : `\n${failed} of ${checks.length} failed`)
process.exit(failed === 0 ? 0 : 1)
