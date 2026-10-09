import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildManifest, names } from './release-manifest.mjs'

const files = ['agent-moshpit_windows_x64-setup.exe', 'agent-moshpit_darwin_aarch64.app.tar.gz', 'agent-moshpit_darwin_x64.app.tar.gz', 'agent-moshpit_linux_amd64.AppImage']
const assets = files.flatMap((name, i) => [{ name, id: i * 2 }, { name: `${name}.sig`, id: i * 2 + 1 }])

test('a manifest retry uses the same retained signature inputs', () => {
  const build = list => buildManifest('owner/repo', 'v0.3.0', list, asset => `signed-${asset.id}`, '2026-10-09T00:00:00Z')
  const first = build(assets)
  const retry = build([...assets, { name: 'latest.json', id: 99 }])
  assert.deepEqual(retry, first)
  assert.equal(first.platforms['windows-x86_64'].signature, 'signed-1')
  assert.equal(first.platforms['darwin-aarch64'].url, 'https://github.com/owner/repo/releases/download/v0.3.0/agent-moshpit_darwin_aarch64.app.tar.gz')
})

test('an incomplete release cannot publish a manifest', () => {
  assert.throws(() => buildManifest('owner/repo', 'v0.3.0', assets.slice(2), () => 'signature'), /No signed update file for: windows-x86_64/)
  assert.deepEqual(names('unrecognized.zip'), [])
})
