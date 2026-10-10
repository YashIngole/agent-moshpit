import assert from 'node:assert/strict'
import test from 'node:test'
import { versionProblems } from './release-check.mjs'

const files = version => ({
  packageJson: JSON.stringify({ name: 'agent-moshpit', version }),
  cargoToml: `[package]\nname = "agent-moshpit"\nversion = "${version}"\n\n[dependencies]\nserde = { version = "1" }\n`,
  tauriConf: JSON.stringify({ productName: 'Agent Moshpit', version })
})

test('a tag matching every version file can ship', () => {
  assert.deepEqual(versionProblems('v0.5.4', files('0.5.4')), [])
  assert.deepEqual(versionProblems('v1.0.0-beta.1', files('1.0.0-beta.1')), [])
})

test('a tag the app would not report as its own version cannot ship', () => {
  const stale = { ...files('0.5.4'), tauriConf: JSON.stringify({ version: '0.5.3' }) }
  assert.deepEqual(versionProblems('v0.5.4', stale), ['src-tauri/tauri.conf.json says 0.5.3, not 0.5.4.'])
  assert.equal(versionProblems('v0.5.5', files('0.5.4')).length, 3)
  // A dependency's version further down Cargo.toml is not the app's.
  const unversioned = { ...files('0.5.4'), cargoToml: '[package]\nname = "x"\n\n[dependencies]\nversion = "0.5.4"\n' }
  assert.deepEqual(versionProblems('v0.5.4', unversioned), ['src-tauri/Cargo.toml says no version, not 0.5.4.'])
})

test('only version tags are releases', () => {
  assert.equal(versionProblems('v0.1.0-hermes!', files('0.1.0')).length, 1)
  assert.equal(versionProblems('0.5.4', files('0.5.4')).length, 1)
})
