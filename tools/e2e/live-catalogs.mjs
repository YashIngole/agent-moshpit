// Real desktop IPC + real model catalogs, without starting any agent task.
import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { attach, forgetAddress, killTree, launch } from './lib.mjs'

const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-live-catalogs-'))
const { child, port } = launch({ port: 9258, env: { MOSHPIT_DATA_DIR: data, MOSHPIT_INSTANCE: 'live-catalogs' } })
let browser
try {
  const attached = await attach(port)
  browser = attached.browser
  const page = attached.page
  const call = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
  for (const harness of ['claude', 'codex']) {
    const catalog = await call('model_catalog', { harness, cwd: data, profile: '', refresh: true })
    assert.equal(catalog.problem, '')
    assert.ok(catalog.models.length > 0)
    assert.ok(catalog.models.every(model => model.id && model.name && Array.isArray(model.efforts)))
    assert.ok(catalog.permissions.some(mode => mode.id === (harness === 'claude' ? 'bypassPermissions' : 'yolo')))
    assert.ok(catalog.permissions.some(mode => mode.id === 'auto'))
    const cached = await call('model_catalog', { harness, cwd: data, profile: '', refresh: false })
    assert.equal(cached.fetched_ms, catalog.fetched_ms)
    console.log(`ok   ${harness}: ${catalog.models.length} live models, provider permissions, cached IPC response`)
  }
  assert.equal((await call('snapshot')).agents.length, 0, 'catalog discovery must not create a desk or send a task')
  console.log('ok   real CLI discovery finishes without starting an agent or requesting inference')
} finally {
  if (browser) await browser.close().catch(() => {})
  killTree(child.pid)
  forgetAddress('live-catalogs')
  // The temporary path is directly owned by this test.
  rmSync(data, { recursive: true, force: true })
}
