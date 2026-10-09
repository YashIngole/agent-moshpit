// The whole flow in the real desktop app, with a stand-in for the agent's program:
// no Claude Code, no Codex, no model, nothing spent.
//
//   npm run tauri build -- --debug --no-bundle
//   node tools/e2e/terminals.mjs
//
// The stand-in (fake-agent.mjs) is put in the table through `harnesses.json`, the
// same way anyone adds a program of their own. What is checked: an agent is
// seated and its program runs in a terminal in the window, what is typed reaches
// it, its status follows what it does, terminals split, close and come back, and
// its program is a child of the app that dies with it.
//
// Also checked here, because only the real app can show them: the keys a terminal
// keeps (Shift+Tab), the clipboard read without a prompt, a file path opened in the
// editor (a stand-in `cursor` put first on the app's PATH), the question about
// trusting a folder, the address a clicked notification opens, and what a desk
// shows after the office has quit and come back.
//
// The app runs as the named office `e2e`, with its own data folder. The clipboard
// is read once and never written; nothing real is typed or clicked on the desktop.
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { addressCommand, appPath, attach, forgetAddress, isRunning, killTree, launch, notices, openAddress, processes, sleep, windowTitle } from './lib.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const instance = process.env.MOSHPIT_INSTANCE || 'e2e'
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-e2e-'))
const work = mkdtempSync(path.join(os.tmpdir(), 'moshpit-work-'))
// A folder nobody has told the stand-in "Claude" to trust.
const fresh = mkdtempSync(path.join(os.tmpdir(), 'moshpit-fresh-'))
mkdirSync(path.join(work, 'src'))
writeFileSync(path.join(work, 'src', 'app.ts'), 'export {}\n')
// A second stand-in is not installed yet: its own install command puts it in place.
const later = path.join(data, 'later')
const fake = { program: process.execPath, args: [path.join(here, 'fake-agent.mjs'), '--hold-initial-work'], task: 'last' }
writeFileSync(
  path.join(data, 'harnesses.json'),
  JSON.stringify([
    { id: 'fake', name: 'Fake Agent', tag: 'Fake', ...fake },
    { id: 'late', name: 'Late Agent', tag: 'Late', program: path.join(later, 'late.cmd'), task: 'last', install: [process.execPath, path.join(here, 'install-late.mjs'), later] },
    // Asks about its folder first, the way Claude Code does: the actual dialog must be visible before its desk raises a hand.
    { id: 'asker', name: 'Asker Agent', tag: 'Asker', ...fake, args: [...fake.args, `--trust-except=${work}`], trust: 'claude' },
    { id: 'probe', name: 'Key Probe', tag: 'Probe', program: process.execPath, args: [path.join(here, 'key-probe.mjs')] }
  ])
)
// The pretend Claude Code has been told to trust the work folder, and no other.
const claudeHome = path.join(data, 'claude-home')
mkdirSync(claudeHome)
writeFileSync(path.join(claudeHome, '.claude.json'), JSON.stringify({ projects: { [work.replaceAll('\\', '/')]: { hasTrustDialogAccepted: true } } }))
// A stand-in editor, first on the app's PATH: it writes down what it was asked to open.
const bin = path.join(data, 'bin')
const opened = path.join(bin, 'opened.txt')
mkdirSync(bin)
writeFileSync(path.join(bin, 'cursor.cmd'), `@echo off\r\necho %* > "${opened}"\r\n`)
const pathKey = Object.keys(process.env).find(key => key.toLowerCase() === 'path') ?? 'PATH'

// A pretend place for the office to ask about newer versions of itself: this computer,
// saying that version 99 is out. What is asked for is counted, and the installer it
// names must never be fetched, because nobody here asks for the update.
const fetched = { manifest: 0, installer: 0 }
const releases = createServer((request, response) => {
  if (request.url?.startsWith('/latest.json')) {
    fetched.manifest += 1
    const { port } = releases.address()
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end(JSON.stringify({ version: '99.0.0', notes: 'A pretend newer version, for the test.', pub_date: new Date().toISOString(), platforms: { 'windows-x86_64': { signature: 'not a signature', url: `http://127.0.0.1:${port}/installer.exe` } } }))
  } else {
    fetched.installer += 1
    response.writeHead(404).end()
  }
})
await new Promise(resolve => releases.listen(0, '127.0.0.1', resolve))
const env = { MOSHPIT_DATA_DIR: data, CLAUDE_CONFIG_DIR: claudeHome, MOSHPIT_UPDATE_URL: `http://127.0.0.1:${releases.address().port}/latest.json`, [pathKey]: `${bin};${process.env[pathKey] ?? ''}` }

let failed = 0
function check(what, ok, detail = '') {
  if (!ok) failed += 1
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${ok || detail === '' ? '' : `  (${detail})`}`)
}

/** Every process under the app. */
const children = pid => processes(pid)
const nodes = pid => children(pid).filter(row => row.name.toLowerCase().includes('node'))
const until = async (ready, ms = 15000) => {
  const end = Date.now() + ms
  while (Date.now() < end) {
    if (await ready()) return true
    await sleep(150)
  }
  return false
}

let { child, port } = launch({ port: 9241, env })
let fakes = []
try {
  let { browser, page } = await attach(port)
  await page.waitForSelector('.floor')

  /** The text on a pane's terminal, read from what xterm keeps for screen readers. */
  const screen = title => page.locator('.pane', { hasText: title }).locator('.xterm-accessibility-tree').innerText()
  const shows = (title, text, ms) => until(async () => (await screen(title).catch(() => '')).includes(text), ms)
  const type = async (title, text) => {
    await page.locator('.pane', { hasText: title }).locator('.term-host').click()
    await page.keyboard.type(text)
    await page.keyboard.press('Enter')
  }
  const desk = title => page.locator('.floor button[data-desk]', { hasText: title })
  const state = async title => (await desk(title).getAttribute('class')) ?? ''
  /** Seat a stand-in through the form, as a person would. */
  const seat = async ({ program = 'Fake Agent', task = '', folder = work, name }) => {
    await page.getByRole('button', { name: 'New agent' }).first().click()
    await page.getByRole('radiogroup').getByText(program).click()
    if (task) await page.getByLabel(/What should they do/).fill(task)
    await page.getByRole('textbox', { name: 'Folder' }).fill(folder)
    await page.getByLabel(/^Name/).fill(name)
    await page.getByRole('button', { name: 'Start agent' }).click()
  }

  // ── a named office says so, and reads the clipboard without asking ───────
  check('a named office says its name in the window title', windowTitle(child.pid).startsWith(`Agent Moshpit (${instance})`), windowTitle(child.pid))
  check('the window may read the clipboard without a prompt', await until(async () => (await page.evaluate(() => navigator.permissions.query({ name: 'clipboard-read' }).then(p => p.state))) === 'granted', 6000), await page.evaluate(() => navigator.permissions.query({ name: 'clipboard-read' }).then(p => p.state)))
  // A window driven by a test is not always in front; the page is told it is, as it would be when used.
  const session = await page.context().newCDPSession(page)
  await session.send('Emulation.setFocusEmulationEnabled', { enabled: true }).catch(() => {})
  // Read, never written, and what is read is not looked at: a prompt would leave this waiting.
  const read = await page.evaluate(() => Promise.race([navigator.clipboard.readText().then(() => 'read', error => `refused: ${error.name}`), new Promise(resolve => setTimeout(() => resolve('left waiting'), 4000))]))
  check('and a read of it comes straight back', read === 'read', read)

  // ── an agent is seated, with a task ──────────────────────────────────────
  await page.getByRole('button', { name: 'New agent' }).first().click()
  check('a program added to the table by hand is offered', (await page.getByRole('radio', { name: 'Fake Agent' }).count()) === 1)
  await page.keyboard.press('Escape')
  await seat({ task: 'say hello & goodbye', name: 'First' })

  check('their terminal opens in the window', await until(async () => (await page.locator('.pane').count()) === 1))
  check('the program runs in it, in the folder chosen', await shows('First', `FAKE AGENT READY in ${work}`), await screen('First').catch(e => String(e)))
  check('the task reached it whole, as one word', await shows('First', 'TASK: say hello & goodbye'))
  check('they have a desk on the floor, tagged with their program', (await desk('First').locator('.tag').textContent()) === 'Fake')
  check('while it prints, they are working', await until(async () => (await state('First')).includes('working'), 4000))
  // Release the fixture only after observing its working state, so a busy
  // machine cannot finish the short turn before the assertion gets to read it.
  await type('First', 'finish')
  check('it says it has finished', await shows('First', 'FINISHED'))
  // Idle when the window is in front, which a window driven by a test may not be; a flag otherwise.
  check('and then they are no longer working', await until(async () => /idle|done/.test(await state('First')), 8000), await state('First'))

  // ── typing reaches it; a question raises a hand ──────────────────────────
  await type('First', 'ping')
  check('what is typed reaches the program', await shows('First', 'YOU SAID: ping'))
  check('silence without a completion notice stays quiet rather than done', await until(async () => (await state('First')).includes('quiet'), 6000), await state('First'))
  await page.locator('.pane', { hasText: 'First' }).getByRole('button', { name: /^More for/ }).click()
  await page.getByRole('menuitem', { name: /Stop their program/ }).click()
  check('stopping a quiet program still asks first', await page.getByRole('button', { name: 'Keep it' }).isVisible())
  await page.getByRole('button', { name: 'Keep it' }).click()
  await type('First', 'size')
  await shows('First', 'SIZE ')
  const size = /SIZE (\d+)x(\d+)/.exec(await screen('First'))
  check('the program knows the size of its pane', size !== null && Number(size[1]) > 40 && Number(size[2]) > 10, size?.[0])
  await type('First', 'ask')
  check('a bell from the program raises a hand', await until(async () => (await state('First')).includes('needs_you'), 6000))
  check('the waiting band stays visible even with their terminal open', (await page.locator('.band').count()) === 1)
  // Going back to the floor is not an answer: the pane losing focus must not lower the hand.
  await page.keyboard.press('Control+Backquote')
  await sleep(2500)
  check('back on the floor, the hand is still up', (await state('First')).includes('needs_you'), await state('First'))
  check('and the band says so', (await page.locator('.band').count()) === 1)
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await page.locator('.pane').count()) === 1)
  await type('First', 'yes')
  check('answering lowers it', await until(async () => !(await state('First')).includes('needs_you'), 6000))

  // ── a file path a program names opens in the editor that is here ─────────
  await type('First', 'path')
  await shows('First', 'EDITED src/app.ts:42')
  const pane = page.locator('.pane', { hasText: 'First' })
  const line = await pane.locator('.xterm-accessibility-tree > div', { hasText: 'EDITED src/app.ts:42' }).first().boundingBox()
  const across = await pane.locator('.xterm-screen').boundingBox()
  let onLink = false
  for (let x = across.x + 30; x < across.x + across.width && !onLink; x += 4) {
    await page.mouse.move(x, line.y + line.height / 2)
    onLink = (await pane.locator('.xterm-cursor-pointer').count()) > 0
  }
  check('a file path in a terminal is a link', onLink)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
  const asked = () => (existsSync(opened) ? readFileSync(opened, 'utf8') : '')
  // The editor receives a canonical path. Windows CI may give Node an 8.3
  // TEMP alias, so compare the actual file path rather than that alias's spelling.
  const expectedFile = `${realpathSync.native(path.join(work, 'src', 'app.ts'))}:42`
  check('Ctrl and a click opens it in the editor on this computer, at its line', await until(() => asked().includes('-g') && asked().toLowerCase().includes(expectedFile.toLowerCase()), 8000), `expected ${expectedFile}; got ${asked()}`)
  await page.mouse.move(across.x + 10, across.y + across.height - 10)

  // ── a second agent; panes split, close and come back ─────────────────────
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('radiogroup').getByText('Fake Agent').click()
  // The folder used a moment ago is offered as a button.
  await page.getByRole('group', { name: 'Folders used before' }).getByRole('button').first().click()
  await page.getByLabel(/^Name/).fill('Second')
  await page.getByRole('button', { name: 'Start agent' }).click()
  check('a second agent gets a pane beside the first', await until(async () => (await page.locator('.pane').count()) === 2))
  check('and a program of their own', await shows('Second', 'FAKE AGENT READY'))
  check('with nothing to do, they settle as idle', await until(async () => (await state('Second')).includes('idle'), 8000))
  const widths = await page.locator('.pane').evaluateAll(panes => panes.map(p => Math.round(p.getBoundingClientRect().width)))
  check('two panes share the room evenly', Math.abs(widths[0] - widths[1]) <= 2, widths.join(' and '))

  fakes = nodes(child.pid)
  check('each program is a process of its own, under the app', fakes.length === 2, children(child.pid).map(r => r.name).join(', '))

  // The first works unwatched: its terminal is put away, and its end raises a flag.
  await type('First', 'work')
  await page.keyboard.press('Control+Backquote')
  check('Ctrl+` goes back to the floor', await until(async () => (await page.locator('.pane').count()) === 0))
  check('what it prints unwatched puts a dot on its desk', await until(async () => (await desk('First').locator('.unread').count()) === 1, 6000))
  check('unwatched work ends with a flag', await until(async () => (await state('First')).includes('done'), 9000), await state('First'))
  // Read back from the notification centre: what a click on it opens is this desk's own address.
  const firstId = await desk('First').getAttribute('data-desk')
  const told = () => notices(instance).filter(xml => xml.includes('First is done'))
  check('and a notification that opens their desk when it is clicked', await until(() => told().some(xml => xml.includes(`launch="agent-moshpit-${instance}://desk/${firstId}" activationType="protocol"`)), 10000), notices(instance).join(' | '))
  check('one for the desk, not one for every time', told().length === 1, String(told().length))
  // CDP can send keys to a background webview; reading requires native window focus.
  openAddress(`agent-moshpit-${instance}://open`)
  await until(() => page.evaluate(() => document.hasFocus()), 5000)
  await page.keyboard.press('Control+Backquote')
  check('and again brings the terminals back', await until(async () => (await page.locator('.pane').count()) === 2))
  check('with what was on them, though the panes were gone', await shows('First', 'YOU SAID: ping'))
  check('looking at them takes the flag down', await until(async () => !(await state('First')).includes('done'), 6000))
  check('and the dot off', await until(async () => (await desk('First').locator('.unread').count()) === 0, 6000))

  await page.locator('.pane', { hasText: 'Second' }).getByRole('button', { name: 'Put this terminal away' }).click()
  check('a pane can be put away on its own', await until(async () => (await page.locator('.pane').count()) === 1))
  check('its program keeps running', nodes(child.pid).length === 2)
  await desk('Second').click({ modifiers: ['Control'] })
  check('Ctrl and a click on a desk opens it beside the others', await until(async () => (await page.locator('.pane').count()) === 2))

  // ── stopping, and starting again ─────────────────────────────────────────
  await page.locator('.pane', { hasText: 'Second' }).getByRole('button', { name: /^More for/ }).click()
  await page.getByRole('menuitem', { name: /Stop their program/ }).click()
  check('a stopped program leaves its desk, away', await until(async () => (await state('Second')).includes('asleep'), 8000), await state('Second'))
  await page.locator('.pane', { hasText: 'Second' }).getByRole('button', { name: 'Start again' }).click()
  check('and starts again in the same pane', await until(async () => (await state('Second')).match(/idle|starting|working/) !== null, 8000))

  // ── renaming, and starting again from the menu ───────────────────────────
  await desk('Second').focus()
  await page.keyboard.press('F2')
  await page.getByRole('dialog', { name: /Rename/ }).getByRole('textbox').fill('Second, renamed')
  await page.keyboard.press('Enter')
  check('a desk renamed from the window is renamed in the core', await until(async () => (await desk('Second, renamed').count()) === 1))
  await type('Second, renamed', 'before the restart')
  await shows('Second, renamed', 'YOU SAID: before the restart')
  // Someone still starting, or at work, is asked about first; at rest, Restart just restarts.
  await until(async () => (await state('Second, renamed')).includes('idle'), 8000)
  await page.locator('.pane', { hasText: 'Second, renamed' }).getByRole('button', { name: /^More for/ }).click()
  await page.getByRole('menuitem', { name: /Restart their program/ }).click()
  check(
    'Restart starts their program again in the same pane',
    await until(async () => {
      const text = await screen('Second, renamed')
      return text.includes('FAKE AGENT READY') && !text.includes('before the restart')
    }, 10000)
  )
  await desk('Second, renamed').focus()
  await page.keyboard.press('F2')
  await page.getByRole('dialog', { name: /Rename/ }).getByRole('textbox').fill('')
  await page.keyboard.press('Enter')
  const given = `Fake in ${path.basename(work)}`
  check('a name emptied goes back to the one the desk was given', await until(async () => (await desk(given).count()) === 1, 6000), (await page.locator('.floor .name').allTextContents()).join(' | '))
  await page.locator('.pane', { hasText: given }).getByRole('button', { name: 'Put this terminal away' }).click()
  await until(async () => (await page.locator('.pane').count()) === 1)

  // ── a question about trusting the folder is a raised hand, task or no task ─
  await seat({ program: 'Asker Agent', name: 'Trusted' })
  check('in a folder its program was told to trust, someone with nothing to do settles as idle', await until(async () => (await state('Trusted')).includes('idle'), 8000), await state('Trusted'))
  await page.locator('.pane', { hasText: 'Trusted' }).getByRole('button', { name: 'Put this terminal away' }).click()
  await seat({ program: 'Asker Agent', folder: fresh, name: 'Asks' })
  check('in one it was not, they need you, with no task at all', await until(async () => (await state('Asks')).includes('needs_you'), 8000), await state('Asks'))
  check('and say what for', ((await desk('Asks').locator('.doing').textContent()) ?? '') === 'Asks whether to trust this folder')
  await sleep(3000)
  check('for as long as it goes unanswered', (await state('Asks')).includes('needs_you'), await state('Asks'))
  await type('Asks', 'yes')
  check('and the first key lowers the hand', await until(async () => !(await state('Asks')).includes('needs_you'), 6000), await state('Asks'))
  await page.locator('.pane', { hasText: 'Asks' }).getByRole('button', { name: 'Put this terminal away' }).click()

  // ── Shift+Tab is the program's, and the keyboard stays in the terminal ───
  await seat({ program: 'Key Probe', name: 'Keys' })
  await shows('Keys', 'KEY PROBE READY')
  await page.locator('.pane', { hasText: 'Keys' }).locator('.term-host').click()
  await page.keyboard.press('Shift+Tab')
  check('Shift+Tab reaches the program as ESC [ Z', await shows('Keys', 'KEY 1b5b5a', 6000), await screen('Keys').catch(() => ''))
  check('and the keyboard stays in the terminal', await page.evaluate(() => document.activeElement?.closest('.term-host') !== null), await page.evaluate(() => document.activeElement?.outerHTML.slice(0, 80)))
  await page.keyboard.type('n')
  check('so the next key is the program’s too', (await shows('Keys', 'KEY 6e', 6000)) && (await page.locator('aside.panel').count()) === 0)
  await page.locator('.pane', { hasText: 'Keys' }).getByRole('button', { name: 'Put this terminal away' }).click()

  // ── the address a clicked notification opens brings up that desk ─────────
  const command = addressCommand(instance)
  check('Windows is told that the office opens its own address', command.includes('agent-moshpit.exe') && command.includes(`--instance ${instance}`) && command.includes('%1'), command.trim().split('\n').pop())
  await page.keyboard.press('Control+Backquote')
  await until(async () => (await page.locator('.pane').count()) === 0)
  const id = await desk('Asks').getAttribute('data-desk')
  openAddress(`agent-moshpit-${instance}://desk/${id}`)
  check('opening a desk’s address opens that desk’s terminal in the running office', await until(async () => (await page.locator('.pane h2', { hasText: 'Asks' }).count()) === 1, 15000), (await page.locator('.pane h2').allTextContents()).join(' | '))
  check('beside the terminals that were open', (await page.locator('.pane').count()) >= 2)
  check('and starts no second office', (await until(async () => processes(child.pid).filter(row => row.name.toLowerCase().includes('agent-moshpit')).length === 1, 6000)) && isRunning(child.pid))

  // ── a program that is not here yet: installed in a pane, then started ────
  await page.getByRole('button', { name: 'New agent' }).first().click()
  await page.getByRole('button', { name: /to install$/ }).click()
  await page.getByRole('radiogroup').getByText('Late Agent').click()
  check('what installs it is shown first', ((await page.locator('code.line').textContent()) ?? '').includes('install-late.mjs'))
  await page.getByLabel(/What should they do/).fill('arrive late')
  await page.getByRole('textbox', { name: 'Folder' }).fill(work)
  await page.getByRole('button', { name: 'Install Late Agent and start' }).click()
  check('the install runs in a pane, where it can be watched', await shows('Late Agent', 'INSTALLING LATE AGENT'))
  check('then the agent starts in that same pane', await until(async () => (await page.locator('.pane h2', { hasText: 'arrive late' }).count()) === 1, 20000))
  check('and its program is the one just installed', await shows('arrive late', 'TASK: arrive late'), JSON.stringify(await page.locator('.pane').evaluateAll(ps => ps.map(p => [p.getAttribute('aria-label'), (p.querySelector('.xterm-accessibility-tree')?.innerText ?? '').slice(0, 400)]))))
  check('the install leaves no pane behind', (await page.locator('.pane[aria-label*="Late Agent"]').count()) === 0 || (await page.locator('.pane[aria-label*="installing"]').count()) === 0)

  // ── closing the window can be made to quit; the core keeps the choice ────
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitemcheckbox', { name: /Closing the window quits/ }).click()
  const chosen = () => (existsSync(path.join(data, 'settings.json')) ? readFileSync(path.join(data, 'settings.json'), 'utf8') : '')
  check('"Closing the window quits" is kept by the core', await until(() => /"close_quits":\s*true/.test(chosen()), 5000), chosen())
  await page.getByRole('button', { name: 'More', exact: true }).click()
  await page.getByRole('menuitemcheckbox', { name: /Closing the window quits/ }).click()
  await until(() => /"close_quits":\s*false/.test(chosen()), 5000)

  // ── a newer version of the office itself is offered, and nothing is fetched unasked ──
  await page.getByRole('button', { name: 'More', exact: true }).click()
  if (/[\\/]debug[\\/]/i.test(appPath())) {
    check('a newer version of the office is offered in the menu', await until(async () => (await page.getByRole('menuitem', { name: /Update to 99\.0\.0/ }).count()) === 1, 20000), JSON.stringify(fetched))
    check('saying which version this one is', /You have \d+\.\d+\.\d+/.test((await page.getByRole('menuitem', { name: /Update to 99\.0\.0/ }).textContent().catch(() => '')) ?? ''))
    check('its releases were asked, and nothing was fetched', fetched.manifest >= 1 && fetched.installer === 0, JSON.stringify(fetched))
  } else {
    // A release build asks only over https: the pretend address on this computer is plain http, and is left alone.
    await sleep(12000)
    check('a release build does not ask a plain http address about newer versions', fetched.manifest === 0 && (await page.getByRole('menuitem', { name: /Update to/ }).count()) === 0, JSON.stringify(fetched))
  }
  await page.keyboard.press('Escape')

  // ── the office quits and comes back: desks show where they left off ──────
  await until(async () => (await page.locator('.floor button.desk').evaluateAll(desks => desks.every(d => !/\b(working|starting|needs_you)\b/.test(d.className)))), 15000)
  fakes = nodes(child.pid)
  await page.evaluate(() => void window.__TAURI_INTERNALS__.invoke('quit')).catch(() => {})
  await browser.close().catch(() => {})
  check('Quit, with nobody busy, ends the office', await until(() => !isRunning(child.pid), 15000))
  check('and its programs with it', await until(() => fakes.every(row => !isRunning(row.pid)), 8000))
  const kept = existsSync(path.join(data, 'screens')) ? readdirSync(path.join(data, 'screens')) : []
  check('each desk’s screen is kept for next time', kept.length >= 4, kept.join(', '))
  const drawn = kept.map(file => readFileSync(path.join(data, 'screens', file), 'utf8')).find(text => text.includes('YOU SAID: ping')) ?? ''
  check('as it was drawn: plain lines, with no cursor moves to land wrong at another size', drawn.includes('FAKE AGENT READY') && !/\x1b\[\d*(;\d*)?[HfABJ]/.test(drawn), JSON.stringify(drawn.slice(0, 200)))

  ;({ child, port } = launch({ port: 9242, env }))
  ;({ browser, page } = await attach(port))
  await page.waitForSelector('.floor button[data-desk]')
  check('started again, everyone is back at a desk, away', await until(async () => (await page.locator('.floor button.desk').count()) >= 5 && (await page.locator('.floor button.desk').evaluateAll(desks => desks.every(d => d.className.includes('asleep'))))), (await page.locator('.floor button.desk').evaluateAll(desks => desks.map(d => d.className))).join(' | '))
  // As it starts, the office asks each program its version, one after another, down the table; the
  // key probe is the last row. Those have ended before any program is counted.
  await until(async () => ((await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('snapshot'))).harnesses.find(h => h.id === 'probe')?.version ?? '') !== '', 60000)
  await until(() => nodes(child.pid).length === 0, 15000)
  await desk('First').click()
  // The terminals that were open come back with it; given the room, its whole screen is in view.
  const first = page.locator('.pane', { hasText: 'First' })
  await first.waitFor()
  if ((await page.locator('.pane').count()) > 1) await first.getByRole('button', { name: 'Give this terminal the room' }).click()
  check('an away desk shows what its terminal last showed', await shows('First', 'YOU SAID: ping', 8000), await screen('First').catch(e => String(e)))
  const lines = (await screen('First')).split('\n').map(text => text.trim()).filter(Boolean)
  const at = text => lines.indexOf(text)
  check(
    'in order, each line once, though its pane changed size many times',
    at('TASK: say hello & goodbye') >= 0 && at('TASK: say hello & goodbye') < at('YOU SAID: ping') && at('YOU SAID: ping') < at('EDITED src/app.ts:42') && lines.filter(text => text === 'YOU SAID: ping').length === 1 && lines.filter(text => text === 'EDITED src/app.ts:42').length === 1,
    lines.join(' / ')
  )
  check('without starting their program', (await state('First')).includes('asleep') && nodes(child.pid).length === 0, `${await state('First')}, ${nodes(child.pid).length} programs`)
  check('and says it is not running, with the way to start it', (await first.locator('.ended').count()) === 1 && (await first.getByRole('button', { name: 'Start again' }).count()) === 1)

  // ── the programs die with the app ────────────────────────────────────────
  await first.getByRole('button', { name: 'Start again' }).click()
  check('Start again starts it, in the same pane', await shows('First', 'FAKE AGENT READY', 10000))
  fakes = nodes(child.pid)
  check('as a process under the app', fakes.length === 1, children(child.pid).map(r => r.name).join(', '))
  await browser.close().catch(() => {})
  // Only the app itself is ended, as a crash would end it.
  execFileSync('taskkill', ['/PID', String(child.pid), '/F'], { stdio: 'ignore' })
  check('when the app dies, its programs die with it', await until(async () => fakes.every(row => !isRunning(row.pid)), 8000), fakes.filter(row => isRunning(row.pid)).map(row => row.pid).join(', '))
} catch (error) {
  failed += 1
  console.log(`FAIL the test itself broke: ${error?.stack ?? error}`)
} finally {
  killTree(child.pid)
  for (const row of fakes) killTree(row.pid)
  releases.close()
  // Nothing of the test is left behind: not its address in the registry, not its folders.
  forgetAddress(instance)
  await sleep(300)
  for (const folder of [data, work, fresh]) rmSync(folder, { recursive: true, force: true })
}

console.log(failed === 0 ? '\nall passed' : `\n${failed} failed`)
process.exit(failed === 0 ? 0 : 1)
