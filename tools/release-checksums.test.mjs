import assert from 'node:assert/strict'
import test from 'node:test'
import { checksumText, installers, parseChecksums } from './release-checksums.mjs'

const hash = c => c.repeat(64)

test('the list names its release and every installer, in a stable order', () => {
  const text = checksumText('v0.5.4', [{ name: 'b.deb', sha256: hash('b') }, { name: 'a-setup.exe', sha256: hash('a') }])
  assert.equal(text, `# Agent Moshpit v0.5.4\n${hash('a')}  a-setup.exe\n${hash('b')}  b.deb\n`)
  const { tag, files } = parseChecksums(text)
  assert.equal(tag, 'v0.5.4')
  assert.deepEqual([...files], [['a-setup.exe', hash('a')], ['b.deb', hash('b')]])
})

test('signatures, the update manifest and the list itself are not installers', () => {
  const names = ['x.dmg', 'x.app.tar.gz', 'x.app.tar.gz.sig', 'latest.json', 'SHA256SUMS', 'SHA256SUMS.sig', 'x-setup.exe']
  assert.deepEqual(installers(names.map(name => ({ name }))).map(a => a.name), ['x.dmg', 'x.app.tar.gz', 'x-setup.exe'])
})

test('nothing malformed is written or read', () => {
  assert.throws(() => checksumText('main', [{ name: 'a', sha256: hash('a') }]))
  assert.throws(() => checksumText('v1.0.0', []))
  assert.throws(() => checksumText('v1.0.0', [{ name: 'a b', sha256: hash('a') }]))
  assert.throws(() => checksumText('v1.0.0', [{ name: 'a', sha256: 'short' }]))
  assert.throws(() => parseChecksums(`${hash('a')}  a\n`))
  assert.throws(() => parseChecksums(`# Agent Moshpit v1.0.0\n${hash('a')} a\n`))
})
