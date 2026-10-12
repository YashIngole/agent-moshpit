// The report of download counts (tools/download-counts.mjs): what is added up, and what is
// taken off GitHub's own count.
import assert from 'node:assert/strict'
import test from 'node:test'
import { github, table, tally } from './download-counts.mjs'

test('each file has its count for each way of asking, and people are the three that want the file', () => {
  const lines = tally([
    { file: 'agent-moshpit_windows_x64-setup.exe', via: 'page', n: 40 },
    { file: 'agent-moshpit_windows_x64-setup.exe', via: 'direct', n: 5 },
    { file: 'agent-moshpit_windows_x64-setup.exe', via: 'bot', n: 12 },
    { file: 'install.sh', via: 'script', n: 9 },
    { file: 'install.sh', via: 'read', n: 3 },
    { file: 'install.sh', via: 'bot', n: 1 },
    // A way of asking a later version counts is not mistaken for people.
    { file: 'install.sh', via: 'something-new', n: 100 }
  ])
  assert.deepEqual(lines, [
    { file: 'agent-moshpit_windows_x64-setup.exe', page: 40, direct: 5, script: 0, read: 0, bot: 12, people: 45 },
    { file: 'install.sh', page: 0, direct: 0, script: 9, read: 3, bot: 1, people: 9 }
  ])
  assert.deepEqual(tally([]), [])
})

test("GitHub's count is added up over releases, without the pipeline's one fetch of each file", () => {
  const release = (draft, counts) => ({ draft, assets: Object.entries(counts).map(([name, download_count]) => ({ name, download_count })) })
  const lines = github([
    release(false, { 'agent-moshpit_windows_x64-setup.exe': 11, 'agent-moshpit_windows_x64-setup.exe.sig': 30, 'agent-moshpit_linux_amd64.deb': 1, 'latest.json': 400, SHA256SUMS: 9, 'SHA256SUMS.sig': 9 }),
    release(false, { 'agent-moshpit_windows_x64-setup.exe': 4, 'agent-moshpit_linux_amd64.deb': 0 }),
    // A draft has been seen by nobody but the pipeline.
    release(true, { 'agent-moshpit_windows_x64-setup.exe': 1 })
  ])
  assert.deepEqual(lines, [
    { file: 'windows_x64-setup.exe', fetched: 13, releases: 2 },
    { file: 'linux_amd64.deb', fetched: 0, releases: 2 }
  ])
})

test('a table lines its columns up, names to the left and numbers to the right', () => {
  assert.equal(table(['', 'people', 'bot'], [['install.sh', 9, 1], ['all', 120, 13]]), ['            people  bot', 'install.sh       9    1', 'all            120   13'].join('\n'))
})
