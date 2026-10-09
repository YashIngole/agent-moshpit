// Isolated Windows desktop + actual stdio bridge + both native launch adapters.
// No model, account, credentials or user office is used.
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { attach, launch, killTree, forgetAddress, sleep } from './lib.mjs'

const root = fileURLToPath(new URL('../../', import.meta.url))
process.env.MOSHPIT_APP ||= path.join(root, 'src-tauri/target/debug/agent-moshpit.exe')
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-mcp-desktop-'))
const work = path.join(data, 'project')
mkdirSync(work)
const report = path.join(data, 'report.jsonl')
writeFileSync(report, '')
const fake = fileURLToPath(new URL('mcp-agent.mjs', import.meta.url))
writeFileSync(path.join(data, 'harnesses.json'), JSON.stringify([
  { id: 'codex', name: 'Codex fixture', tag: 'Codex', program: process.execPath, args: [fake], task: 'last', status: 'activity', session: 'unknown', session_arg: '', resume: [], trust: 'none', package: '', update: [] },
  { id: 'claude', name: 'Claude fixture', tag: 'Claude', program: process.execPath, args: [fake], task: 'last', status: 'activity', session: 'unknown', session_arg: '', resume: [], trust: 'none', package: '', update: [] }
]))
const records = () => readFileSync(report, 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line))
const until = async (ready, ms = 35000) => {
  const deadline = Date.now() + ms
  while (Date.now() < deadline) { if (await ready()) return; await sleep(100) }
  throw new Error(`Timed out. Agent events: ${JSON.stringify(records())}`)
}
let app, browser, page
try {
  app = launch({ port: 9264, env: { MOSHPIT_INSTANCE: 'mcp-e2e', MOSHPIT_DATA_DIR: data, MOSHPIT_MCP_TEST_REPORT: report, MOSHPIT_UPDATE_URL: 'http://127.0.0.1:1/latest.json' } })
  ;({ browser, page } = await attach(9264))
  await page.waitForSelector('.floor')
  const invoke = (command, args) => page.evaluate(async ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  assert.deepEqual(await invoke('startup_problems'), [])
  const initial = await invoke('snapshot')
  assert.equal(initial.harnesses.find(h => h.id === 'codex').name, 'Codex fixture')
  assert.equal(initial.harnesses.find(h => h.id === 'claude').name, 'Claude fixture')
  const parent = await invoke('new_agent', { spec: { harness: 'codex', cwd: work, prompt: 'Parent MCP integration scenario', title: '', worktree: false }, cols: 100, rows: 30 })
  await until(() => records().some(row => row.event === 'delegation-passed'))
  const passed = records().find(row => row.event === 'delegation-passed')
  assert.equal(passed.id, parent)
  assert.equal(path.resolve(passed.cwd).toLowerCase(), path.resolve(work).toLowerCase())
  assert.ok(records().some(row => row.event === 'connected' && row.id === passed.childId && path.resolve(row.cwd).toLowerCase() === path.resolve(work).toLowerCase()))
  assert.ok(records().some(row => row.event === 'reported' && row.taskId === passed.followupId))
  assert.equal(records().filter(row => row.event === 'connected').length, 2)
  await until(() => page.locator(`[data-desk="${parent}"]`).textContent().then(text => text.includes('Coordinate API review')))
  await until(() => page.locator(`[data-desk="${passed.childId}"]`).textContent().then(text => text.includes('Delegated task completed')))
  console.log('ok: Codex-to-Claude delegation, same directory, idempotent launch, inbox follow-up and explicit result handoff')
  const locked = await invoke('new_agent', { spec: { harness: 'claude', cwd: work, prompt: 'Protected name scenario', title: 'User chosen title', worktree: false }, cols: 80, rows: 24 })
  await until(() => records().some(row => row.event === 'connected' && row.id === locked))
  assert.equal(records().find(row => row.id === locked && row.event === 'connected').protectedName, true)
  assert.match(await page.locator(`[data-desk="${locked}"]`).textContent(), /User chosen title/)
  console.log('ok: user names remain protected while agent activity appears on the desk')
  await invoke('stop', { agent: passed.childId })
  await until(async () => (await invoke('snapshot')).agents.find(a => a.id === passed.childId).running === false)
  assert.equal(records().filter(row => row.event === 'error').length, 0)
  await invoke('wake', { agent: passed.childId, cols: 80, rows: 24 })
  await until(() => records().filter(row => row.event === 'connected' && row.id === passed.childId).length === 2)
  console.log('ok: waking a stopped session creates a fresh MCP connection')
  const journal = JSON.parse(readFileSync(path.join(data, 'coordination.json'), 'utf8'))
  assert.equal(journal.tasks.length, 2)
  assert.ok(journal.tasks.every(t => t.status === 'completed'))
  console.log('ok: task results persist on disk without persisting capabilities')
} finally {
  await browser?.close().catch(() => {})
  if (app?.child.pid) killTree(app.child.pid)
  forgetAddress('mcp-e2e')
  // Only this fixture's explicitly created temporary directory is removed.
  assert.ok(path.resolve(data).startsWith(path.resolve(os.tmpdir()) + path.sep))
  assert.ok(path.basename(data).startsWith('moshpit-mcp-desktop-'))
  rmSync(data, { recursive: true, force: true })
}
