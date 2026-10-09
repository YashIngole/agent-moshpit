// Checks of the interface on its own, in a headless browser with the pretend office.
// No programs, no desktop shell: this is about what the window does with a keyboard,
// a narrow floor, a new arrival, and any number of terminals.
//
//   npm run build && node tools/e2e/ui-demo.mjs
import { spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const PORT = 4174
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

async function open(query, width = 1280, height = 800) {
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: 'light' })
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
/** The size of every pane, in the order they are laid out. */
const boxes = page => page.locator('.pane').evaluateAll(all => all.map(p => p.getBoundingClientRect()).map(r => ({ x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) })))
const desk = (page, n) => page.locator(`.floor button[data-desk="demo-${n}"]`)

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
  check('every desk can still be reached', (await page.locator('.floor button[data-desk]').count()) === 10)
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

  // A plain click shows another desk in the same pane; Ctrl and a click gives it a pane of its own.
  await desk(page, 5).click()
  check('a plain click shows the next desk in the same pane', (await panes(page)) === 1 && ((await page.locator('.pane h2').textContent()) ?? '') === 'Docs pass for the API')
  await desk(page, 2).click({ modifiers: ['ControlOrMeta'] })
  check('Ctrl and a click opens a second pane', await until(async () => (await panes(page)) === 2))
  let b = await boxes(page)
  check('two panes sit side by side, evenly', b.length === 2 && b[0].y === b[1].y && Math.abs(b[0].w - b[1].w) <= 2, JSON.stringify(b))

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
  await desk(page, 4).click({ modifiers: ['ControlOrMeta'] })
  await until(async () => (await panes(page)) === 3)
  b = await boxes(page)
  check('three panes are two above one', b.filter(p => p.y === b[0].y).length === 2 && b.filter(p => p.y > b[0].y).length === 1, JSON.stringify(b))
  await desk(page, 6).click({ modifiers: ['ControlOrMeta'] })
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

  // ── the floor and back ──
  await page.keyboard.press('Control+Backquote')
  check('Ctrl+` goes back to the floor', await until(async () => (await panes(page)) === 0))
  check('and offers the terminals again', (await page.getByRole('button', { name: /terminals/i }).count()) === 1)
  await page.keyboard.press('Control+Backquote')
  check('Ctrl+` again brings them back', await until(async () => (await panes(page)) === 1))
  await page.getByRole('button', { name: /back to the floor/i }).click()
  await until(async () => (await panes(page)) === 0)

  // ── someone waiting ──
  await page.getByRole('button', { name: 'Open their terminal' }).click()
  check('the band opens the terminal of whoever is waiting', await until(async () => (await page.locator('.pane h2', { hasText: 'Fix the checkout total' }).count()) === 1))
  check('and then has nothing more to say', (await page.locator('.band').count()) === 0)
  await page.keyboard.press('Control+Backquote')
  check('back on the floor, it does again', await until(async () => (await page.locator('.band').count()) === 1))

  // ── a half-written task is not lost ──
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByLabel(/What should they do/).fill('Half a thought')
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'New agent' }).first().click()
  check('a half-written task is still there when the form is opened again', (await page.getByLabel(/What should they do/).inputValue()) === 'Half a thought')
  check('programs on this computer are offered first', (await page.getByRole('radio').count()) === 4 && (await page.getByRole('radio', { name: /OpenCode/ }).count()) === 0)
  await page.getByRole('button', { name: '+2 to install' }).click()
  check('and ones the office can install on asking', (await page.getByRole('radio').count()) === 6 && (await page.getByRole('radio', { name: /OpenCode/ }).count()) === 1)
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
  for (const n of [1, 2, 3, 6, 8]) await desks.nth(n).click({ modifiers: ['ControlOrMeta'] })
  await until(async () => (await panes(page)) === 6)
  b = await boxes(page)
  check('six panes are three by two', new Set(b.map(p => p.x)).size === 3 && new Set(b.map(p => p.y)).size === 2, JSON.stringify(b))
  check('and each is still wide enough to read', Math.min(...b.map(p => p.w)) >= 300, Math.min(...b.map(p => p.w)))
  await page.context().close()

  // ── what a vibe coder reaches for every day (docs/reviews/ux-sweep-2026-10-08.md) ──
  page = await open('demo=office&still')
  await desk(page, 2).click()
  await desk(page, 4).click({ modifiers: ['ControlOrMeta'] })
  await desk(page, 5).click({ modifiers: ['ControlOrMeta'] })
  await until(async () => (await panes(page)) === 3)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 6).click()
  check('picking someone from the floor brings the grid back, with them in it', await until(async () => (await panes(page)) === 3 && (await page.locator('.pane h2', { hasText: 'Bump dependencies' }).count()) === 1))
  await desk(page, 1).focus()
  await page.keyboard.press('Control+Enter')
  check('Ctrl+Enter on a desk opens it beside the others', await until(async () => (await panes(page)) === 4))

  const spot = title => page.locator('.pane', { hasText: title }).evaluate(p => `${Math.round(p.getBoundingClientRect().x)},${Math.round(p.getBoundingClientRect().y)}`)
  const [one, two] = await page.locator('.pane h2').allTextContents()
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
  await desk(page, 3).click()
  check('opening an away desk shows them as they were left, without starting them', await until(async () => (await page.locator('.pane .ended').count()) >= 1) && ((await desk(page, 3).getAttribute('class')) ?? '').includes('asleep'))
  await desk(page, 3).click({ button: 'middle' })
  check('a middle click takes a desk off the floor', await until(async () => (await desk(page, 3).count()) === 0))
  check('with a moment to bring it back', (await page.getByRole('button', { name: 'Undo' }).count()) === 1)
  await page.getByRole('button', { name: 'Undo' }).click()
  check('and Undo does', await until(async () => (await desk(page, 3).count()) === 1))
  check('a desk on the floor can be taken away from its corner', (await page.getByRole('button', { name: 'Remove Flaky test hunt' }).count()) === 1)

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
  await page.keyboard.type('src')
  check('and says when it has found it', ((await page.locator('.find .hint').textContent()) ?? '') !== 'not found')
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
  check('a file dropped on a pane is pasted into it as its path', await until(async () => ((await page.locator('.pane', { hasText: targetTitle }).locator('.xterm-accessibility-tree').innerText()) ?? '').replace(/\n/g, '').includes('"C:/Users/you/Desktop/screenshot 1.png"')))

  await page.locator('.grip').dblclick()
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
  check('and its band is one line', ((await page.locator('.band').boundingBox())?.height ?? 999) < 70)
  await page.context().close()

  // ── the second sweep (docs/reviews/ux-sweep-2026-10-09.md) ──
  page = await open('demo=office&still')
  const typed = () => page.evaluate(() => window.__demo.typed.map(t => t.data).join(''))
  await desk(page, 1).click()
  await desk(page, 2).click({ modifiers: ['ControlOrMeta'] })
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
  await desk(page, 2).click({ modifiers: ['ControlOrMeta'] })
  await page.locator('.grip').dblclick()
  await until(async () => (await page.locator('.seat.listed').count()) > 0)
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
  check('an away desk says opening it shows where they left off', ((await desk(page, 3).locator('.doing').textContent()) ?? '') === 'Not running. Open to see where they left off.')
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
  check('a desk with something new in its terminal wears a dot', (await desk(page, 5).locator('.unread').count()) === 1 && (await desk(page, 3).locator('.unread').count()) === 0)
  await desk(page, 5).click()
  await until(async () => (await panes(page)) === 1)
  await page.keyboard.press('Control+Backquote')
  check('and loses it once its terminal has been looked at', await until(async () => (await desk(page, 5).locator('.unread').count()) === 0))
  await desk(page, 2).click({ button: 'right' })
  await page.getByRole('menuitem', { name: /^Start another like this/ }).click()
  check('Start another like this opens the form on the same program', await until(async () => (await page.getByRole('radio', { name: 'Codex' }).isChecked().catch(() => false)) === true))
  check('in the same folder', (await page.getByRole('textbox', { name: 'Folder' }).inputValue()) === 'C:/code/shop')
  await page.keyboard.press('Escape')

  // The scroll position survives a trip to the floor.
  await desk(page, 2).click()
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
  check('closing the window can be made to quit', (await page.getByRole('menuitemcheckbox', { name: /Closing the window quits/ }).getAttribute('aria-checked')) === 'true' && ((await page.locator('.about').textContent()) ?? '').includes('closing the window quits'))
  await page.keyboard.press('Escape')

  // The +N chip lists the terminals behind and switches to one.
  await desk(page, 4).click({ modifiers: ['ControlOrMeta'] })
  await desk(page, 6).click({ modifiers: ['ControlOrMeta'] })
  await until(async () => (await panes(page)) === 3)
  await page.locator('.pane', { hasText: 'Refactor auth middleware' }).getByRole('button', { name: 'Give this terminal the room' }).click()
  await page.getByRole('button', { name: /more terminals behind this one/ }).click()
  check('the +N chip lists the terminals behind the one given the room', JSON.stringify(await page.getByRole('menuitem').evaluateAll(items => items.map(i => i.firstChild.textContent.trim()))) === JSON.stringify(['Type the orders API', 'Bump dependencies', 'Put them all back']))
  await page.getByRole('menuitem', { name: /^Bump dependencies/ }).click()
  check('and a click switches to that one', await until(async () => ((await page.locator('.pane:not(.hidden) h2').textContent()) ?? '') === 'Bump dependencies'))
  await page.context().close()

  // The flow extras.
  page = await open('demo=office&still')
  const working = async () => ((await page.locator('.bar .count', { hasText: 'working' }).textContent()) ?? '').trim()
  const workingBefore = await working()
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Claude Code').click()
  await page.getByRole('textbox', { name: 'Folder' }).fill('C:/code/shop')
  await page.getByRole('button', { name: 'Start agent' }).click()
  await until(async () => ((await page.locator('.floor button[data-desk^="demo-new-"]').getAttribute('class')) ?? '').includes('starting'))
  check('someone still starting is not counted as working', (await working()) === workingBefore, `${workingBefore} / ${await working()}`)
  await page.locator('.pane').getByRole('button', { name: 'Put this terminal away' }).click()
  await until(async () => (await panes(page)) === 0)

  // A grid of two, then the floor, then a plain click on a third desk: one pane makes room, and says so.
  await desk(page, 2).click()
  await desk(page, 4).click({ modifiers: ['ControlOrMeta'] })
  await until(async () => (await panes(page)) === 2)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await panes(page)) === 0)
  await desk(page, 6).click()
  const titles = () => page.locator('.pane h2').allTextContents()
  await until(async () => (await titles()).includes('Bump dependencies'))
  const made = ((await page.locator('.toast').textContent()) ?? '').trim()
  check('a plain click from the floor that replaces a pane says which one made room', /made room for Bump dependencies/.test(made), made)
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
  check('and lists Ctrl+Q, and Ctrl and a click on a file path with the editor’s name', columns.text.includes('Ctrl+Q') && /Ctrl and a click on a file path\s*Open it in Cursor, at its line/.test(columns.text))
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
