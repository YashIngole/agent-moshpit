// Windows desktop regressions with disposable data. No real CLI or model is used.
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { attach, forgetAddress, isRunning, killTree, launch, sleep } from './lib.mjs'

const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-reliability-'))
const file = path.join(data, 'desks.json')
const preserved = path.join(data, 'screens', 'old.screen')
const broken = '[{"id":"recoverable but incomplete"'
mkdirSync(path.dirname(preserved))
writeFileSync(preserved, 'A screen that must survive a corrupt desk file')
writeFileSync(file, broken)
const env = { MOSHPIT_DATA_DIR: data, MOSHPIT_INSTANCE: 'reliability', CODEX_HOME: path.join(data, 'custom-codex-home') }
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([{
  id: 'sessions', name: 'Session Agent', tag: 'Session', program: process.execPath,
  args: [fileURLToPath(new URL('./session-agent.mjs', import.meta.url))],
  session: 'codex_rollouts', resume: ['--resume', '{session}']
}]))
let child
let browser

async function until(ready) {
  for (let n = 0; n < 100; n++) { if (await ready()) return true; await sleep(100) }
  return false
}

async function open() {
  const started = launch({ port: 9257, env })
  child = started.child
  const attached = await attach(started.port)
  browser = attached.browser
  await attached.page.waitForSelector('.floor')
  return attached.page
}

const call = (page, command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
async function quit(page) {
  await call(page, 'quit').catch(() => {})
  await browser.close().catch(() => {})
  assert.ok(await until(() => !isRunning(child.pid)), 'office should exit normally')
}

try {
  let page = await open()
  assert.ok(await until(async () => (await page.locator('body').innerText()).includes('Saved desks could not be read')), 'storage failure is visible')
  assert.ok((await call(page, 'startup_problems')).some(p => p.file === file), 'recovery names the original file')
  await quit(page)
  assert.equal(readFileSync(file, 'utf8'), broken)
  assert.equal(readFileSync(preserved, 'utf8'), 'A screen that must survive a corrupt desk file')
  console.log('ok   unreadable saved desks and screens survive startup and normal quit')

  const projects = ['client', 'personal'].map(name => path.join(data, name, 'shop'))
  for (const folder of projects) mkdirSync(folder, { recursive: true })
  const missing = path.join(data, 'missing-project')
  const desk = (id, cwd) => ({ id, title: id, harness: 'codex', cwd, session: 'old-unverified-session' })
  writeFileSync(file, JSON.stringify([desk('client', projects[0]), desk('personal', projects[1]), desk('missing', missing)]))
  page = await open()
  assert.ok(await until(async () => (await page.locator('.room:not(.waiting)').count()) === 3), 'same-name folders have different rooms')
  const snapshot = await call(page, 'snapshot')
  assert.equal(new Set(snapshot.agents.map(a => a.project)).size, 3)
  assert.ok(snapshot.agents.every(a => !a.resumable && a.resume_note.includes('/resume')), 'old guessed IDs are not auto-resumed')
  const error = await call(page, 'wake', { agent: 'missing', cols: 80, rows: 24 }).then(() => '', String)
  assert.match(error, /folder/i)
  const after = (await call(page, 'snapshot')).agents.find(a => a.id === 'missing')
  assert.equal(after.cwd, missing)
  assert.equal(after.running, false)
  console.log('ok   missing folders cannot launch elsewhere; old IDs stay unverified; same-name projects stay separate')
  const create = title => call(page, 'new_agent', { spec: { harness: 'sessions', cwd: projects[0], prompt: '', title, worktree: false }, cols: 100, rows: 30 })
  const ids = [await create('Session A'), await create('Session B')]
  const savedPair = () => JSON.parse(readFileSync(file, 'utf8')).filter(d => ids.includes(d.id))
  assert.ok(await until(() => savedPair().length === 2 && savedPair().every(d => d.session_verified && d.session)), 'custom CODEX_HOME sessions should resolve')
  const pair = savedPair()
  assert.notEqual(pair[0].session, pair[1].session, 'same-folder desks must have different sessions')
  await call(page, 'restart', { agent: ids[0], cols: 100, rows: 30 })
  assert.ok(await until(() => savedPair().find(d => d.id === ids[0])?.session_verified))
  assert.equal(savedPair().find(d => d.id === ids[0]).session, pair.find(d => d.id === ids[0]).session, 'restart must resume its own ID')
  await call(page, 'term_write', { agent: ids[0], data: '/new\r\n' })
  assert.ok(await until(() => { const d = savedPair().find(d => d.id === ids[0]); return d.session_verified && d.session !== pair.find(p => p.id === ids[0]).session }), 'session switch is verified from the new terminal title')
  for (const id of ids) await call(page, 'stop', { agent: id })
  assert.ok(await until(async () => (await call(page, 'snapshot')).agents.filter(d => ids.includes(d.id)).every(d => !d.running)))
  console.log('ok   same-folder sessions, custom CODEX_HOME, own-session restart and session switching')
  await quit(page)
  assert.equal(JSON.parse(readFileSync(file, 'utf8')).find(a => a.id === 'missing').cwd, missing)
  console.log('all reliability checks passed')
} finally {
  if (browser) await browser.close().catch(() => {})
  if (child) killTree(child.pid)
  forgetAddress('reliability')
  await sleep(500)
  rmSync(data, { recursive: true, force: true })
}
