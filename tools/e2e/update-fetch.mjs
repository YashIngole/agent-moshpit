// Tries a release's signed update file against the key built into the app, in the real
// app, without installing anything. Run it by hand before publishing a release (Windows):
//
//   npm run tauri build -- --debug --no-bundle
//   node tools/e2e/update-fetch.mjs v0.3.0
//
// It takes latest.json and the Windows installer from that release (a draft will do; the
// `gh` command must be signed in), serves them from this computer, and asks the office,
// started as the named office `e2e` and told only to fetch, to update to it. (An office
// told only to fetch takes any version for a newer one: a signature is made for one
// version, so the file can only be offered as the version it is.)
// The office must say the update was fetched and passed its check. Then one byte of the
// installer is changed, and the office must refuse it. Nothing is put in place either time.
import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import os from 'node:os'
import path from 'node:path'
import { attach, forgetAddress, killTree, launch, sleep } from './lib.mjs'

const tag = process.argv[2]
if (!tag) {
  console.error('usage: node tools/e2e/update-fetch.mjs <tag>')
  process.exit(2)
}
const repo = 'YashIngole/agent-moshpit'
const FILE = 'agent-moshpit_windows_x64-setup.exe'
const folder = mkdtempSync(path.join(os.tmpdir(), 'moshpit-update-'))
const data = mkdtempSync(path.join(os.tmpdir(), 'moshpit-e2e-'))
const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })

let failed = 0
function check(what, ok, detail = '') {
  if (!ok) failed += 1
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${ok || detail === '' ? '' : `  (${detail})`}`)
}
const until = async (ready, ms = 60000) => {
  const end = Date.now() + ms
  while (Date.now() < end) {
    if (await ready()) return true
    await sleep(200)
  }
  return false
}

gh('release', 'download', tag, '--repo', repo, '--dir', folder, '--pattern', 'latest.json', '--pattern', FILE, '--clobber')
const released = JSON.parse(readFileSync(path.join(folder, 'latest.json'), 'utf8'))
const real = readFileSync(path.join(folder, FILE))
const spoiled = Buffer.from(real)
spoiled[spoiled.length >> 1] ^= 0xff
check(`the release's latest.json names version ${tag.replace(/^v/, '')}`, released.version === tag.replace(/^v/, ''), released.version)
check('and has a signed file for an office installed either way on Windows', Boolean(released.platforms['windows-x86_64']?.signature && released.platforms['windows-x86_64-nsis']?.signature && released.platforms['windows-x86_64-msi']?.signature), Object.keys(released.platforms).join(', '))

let serving = real
const releases = createServer((request, response) => {
  if (request.url?.startsWith('/latest.json')) {
    const { port } = releases.address()
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end(JSON.stringify({ version: released.version, notes: 'The real file, from this computer.', pub_date: new Date().toISOString(), platforms: { 'windows-x86_64': { signature: released.platforms['windows-x86_64'].signature, url: `http://127.0.0.1:${port}/${FILE}` } } }))
  } else {
    response.writeHead(200, { 'content-type': 'application/octet-stream', 'content-length': serving.length })
    response.end(serving)
  }
})
await new Promise(resolve => releases.listen(0, '127.0.0.1', resolve))
const env = { MOSHPIT_DATA_DIR: data, MOSHPIT_UPDATE_URL: `http://127.0.0.1:${releases.address().port}/latest.json`, MOSHPIT_UPDATE_ONLY_FETCH: '1' }

const { child, port } = launch({ port: 9251, env })
let browser
try {
  const attached = await attach(port)
  browser = attached.browser
  const page = attached.page
  const offer = page.getByRole('menuitem', { name: `Update to ${released.version}` })
  // Everything the office says while this runs is kept, so a failure can show what it said instead.
  const heard = []
  const said = async () => {
    const now = ((await page.locator('.toast').textContent().catch(() => '')) ?? '').trim()
    if (now && heard.at(-1) !== now) heard.push(now)
    return now
  }
  const ask = async () => {
    await page.getByRole('button', { name: 'More', exact: true }).click()
    await until(async () => (await offer.count()) === 1, 30000)
    await offer.click()
  }

  await ask()
  check('the real installer is fetched and passes the check against the app\'s key', await until(async () => (await said()).includes('passed its check'), 40000), heard.join(' | '))
  check('and is not put in place', heard.some(words => words.includes('not put in place')))

  serving = spoiled
  await sleep(500)
  await ask()
  heard.length = 0
  check('the same file with one byte changed is refused', await until(async () => (await said()).includes('could not be fetched'), 40000), heard.join(' | '))
  check('for its signature, not for its version', !heard.some(words => words.includes('announced version')), heard.join(' | '))
} catch (error) {
  failed += 1
  console.log(`FAIL the test itself broke: ${error?.stack ?? error}`)
} finally {
  await browser?.close().catch(() => {})
  killTree(child.pid)
  releases.close()
  forgetAddress('e2e')
  await sleep(300)
  for (const gone of [folder, data]) rmSync(gone, { recursive: true, force: true })
}

console.log(failed === 0 ? '\nall passed' : `\n${failed} failed`)
process.exit(failed === 0 ? 0 : 1)
