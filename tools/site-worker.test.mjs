// The site's one piece of code (tools/cloudflare/site.js): an installer asked for at
// /get/<file> is counted and sent on to GitHub, an install script is counted and served,
// and everything else is a file of the site. A count never stands in the way of an answer.
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import worker, { INSTALLERS, SCRIPTS, asked, via } from './cloudflare/site.js'

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const RELEASE = 'https://github.com/YashIngole/agent-moshpit/releases/latest/download/'
const BROWSER = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36'

/** A stand-in for Cloudflare around the Worker: a database that writes down what it is asked, and the site's files. */
function stand(db = 'ok') {
  const rows = []
  const waited = []
  const env = {
    ASSETS: { fetch: request => new Response(`file ${new URL(request.url).pathname}`, { status: new URL(request.url).pathname.startsWith('/get/') ? 404 : 200 }) },
    DB: db === 'none' ? undefined : {
      prepare: sql => ({
        bind: (...values) => ({
          run: async () => {
            if (db === 'broken') throw new Error('D1 is down')
            rows.push({ sql, values })
          }
        })
      })
    }
  }
  const ctx = { waitUntil: promise => waited.push(promise) }
  const ask = async (address, { method = 'GET', headers = {}, country = 'IN' } = {}) => {
    const request = new Request(`https://agentmoshpit.com${address}`, { method, headers })
    Object.defineProperty(request, 'cf', { value: { country } })
    const response = await worker.fetch(request, env, ctx)
    await Promise.all(waited.splice(0))
    return response
  }
  return { ask, rows }
}

test('an installer is counted and sent on to that file of the newest release', async () => {
  const { ask, rows } = stand()
  for (const file of INSTALLERS) {
    const response = await ask(`/get/${file}`, { headers: { 'user-agent': BROWSER, 'sec-fetch-site': 'same-origin', 'sec-fetch-user': '?1' } })
    assert.equal(response.status, 302)
    assert.equal(response.headers.get('location'), RELEASE + file)
    assert.equal(response.headers.get('cache-control'), 'no-store')
  }
  assert.equal(rows.length, INSTALLERS.length)
  const [day, file, how, country] = rows[0].values
  assert.match(day, /^\d{4}-\d{2}-\d{2}$/)
  assert.deepEqual([file, how, country], [INSTALLERS[0], 'page', 'IN'])
  // One row a day for each file, way and country: asked for again, its number goes up.
  assert.match(rows[0].sql, /ON CONFLICT \(day, file, via, country\) DO UPDATE SET count = count \+ 1/)
  // Nothing of who asked is written: four values, none of them an address or a browser.
  assert.equal(rows[0].values.length, 4)
})

test('an install script is counted and then served like any other file', async () => {
  const { ask, rows } = stand()
  const response = await ask('/install.sh', { headers: { 'user-agent': 'curl/8.9.1' }, country: 'DE' })
  assert.equal(response.status, 200)
  assert.equal(await response.text(), 'file /install.sh')
  await ask('/install.ps1', { headers: { 'user-agent': 'Mozilla/5.0 (Windows NT; Windows NT 10.0; en-US) WindowsPowerShell/5.1.26100.4202' } })
  assert.deepEqual(rows.map(row => row.values.slice(1)), [['install.sh', 'script', 'DE'], ['install.ps1', 'script', 'IN']])
})

test('anything else is a file of the site, and is not counted', async () => {
  const { ask, rows } = stand()
  assert.equal(await (await ask('/', { headers: { 'user-agent': BROWSER } })).text(), 'file /')
  assert.equal(await (await ask('/guides/', { headers: { 'user-agent': BROWSER } })).text(), 'file /guides/')
  // A file nobody publishes is not sent on anywhere: it is the site's own "not here".
  for (const address of ['/get/', '/get/agent-moshpit_windows_x64-setup.exe.sig', '/get/https://example.com/x.exe', '/get/install.sh', '/get/latest.json']) {
    const response = await ask(address, { headers: { 'user-agent': BROWSER } })
    assert.equal(response.status, 404, address)
    assert.equal(response.headers.get('location'), null, address)
  }
  assert.equal(rows.length, 0)
})

test('somebody checking that a link works is answered and not counted', async () => {
  const { ask, rows } = stand()
  const response = await ask(`/get/${INSTALLERS[0]}`, { method: 'HEAD', headers: { 'user-agent': BROWSER } })
  assert.equal(response.status, 302)
  assert.equal(rows.length, 0)
})

test('a count that cannot be kept does not stand in the way of the download', async () => {
  for (const db of ['broken', 'none']) {
    const { ask, rows } = stand(db)
    const response = await ask(`/get/${INSTALLERS[0]}`, { headers: { 'user-agent': BROWSER } })
    assert.equal(response.status, 302)
    assert.equal(response.headers.get('location'), RELEASE + INSTALLERS[0])
    assert.equal((await ask('/install.sh', { headers: { 'user-agent': 'curl/8.9.1' } })).status, 200)
    assert.equal(rows.length, 0)
  }
})

test('a country that is not two letters is not written down', async () => {
  const { ask, rows } = stand()
  for (const country of [null, 'T1', 'india', '']) await ask(`/get/${INSTALLERS[0]}`, { headers: { 'user-agent': BROWSER }, country })
  assert.deepEqual(rows.map(row => row.values[3]), ['', '', '', ''])
})

test('how it was asked for is told from what the asker says of itself', () => {
  const how = (file, headers) => via(file, new Headers(headers))
  const exe = INSTALLERS[0]
  assert.equal(how(exe, { 'user-agent': BROWSER, 'sec-fetch-site': 'same-origin' }), 'page')
  // A browser too old to say where a click came from still says which page it was on.
  assert.equal(how(exe, { 'user-agent': BROWSER, referer: 'https://agentmoshpit.com/guides/' }), 'page')
  assert.equal(how(exe, { 'user-agent': BROWSER, 'sec-fetch-site': 'cross-site', referer: 'https://github.com/YashIngole/agent-moshpit' }), 'direct')
  assert.equal(how(exe, { 'user-agent': BROWSER, 'sec-fetch-site': 'none' }), 'direct')
  assert.equal(how(exe, { 'user-agent': 'curl/8.9.1' }), 'direct')
  for (const agent of ['', 'Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)', 'Mozilla/5.0 (X11; Linux x86_64) HeadlessChrome/141.0.0.0', 'python-requests/2.32', 'Slackbot-LinkExpanding 1.0', 'Twitterbot/1.0']) {
    assert.equal(how(exe, { 'user-agent': agent, 'sec-fetch-site': 'same-origin' }), 'bot', agent)
  }
  assert.equal(how('install.sh', { 'user-agent': 'curl/8.9.1' }), 'script')
  assert.equal(how('install.sh', { 'user-agent': 'Wget/1.21.4' }), 'script')
  assert.equal(how('install.ps1', { 'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Microsoft Windows 10.0.26100; en-US) PowerShell/7.5.3' }), 'script')
  assert.equal(how('install.sh', { 'user-agent': BROWSER, 'sec-fetch-dest': 'document' }), 'read')
  assert.equal(how('install.sh', { 'user-agent': 'Mozilla/5.0 (compatible; bingbot/2.0)' }), 'bot')
})

test('what an address asks for', () => {
  assert.deepEqual(asked('/get/agent-moshpit_linux_amd64.deb'), { file: 'agent-moshpit_linux_amd64.deb', to: `${RELEASE}agent-moshpit_linux_amd64.deb` })
  assert.deepEqual(asked('/install.ps1'), { file: 'install.ps1', to: null })
  for (const not of ['/', '/get', '/get/', '/install.sh/', '/guides/install.sh', '/get/install.ps1', '/GET/agent-moshpit_linux_amd64.deb']) assert.equal(asked(not), null, not)
  assert.deepEqual(SCRIPTS, ['install.sh', 'install.ps1'])
})

test('every installer link on the site is one the Worker sends on, and the Worker runs for them', () => {
  const pages = dir => readdirSync(dir, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? (entry.name === 'demo' ? [] : pages(path.join(dir, entry.name))) : entry.name.endsWith('.html') ? [path.join(dir, entry.name)] : [])
  const linked = new Set()
  for (const page of pages(path.join(repo, 'site'))) {
    const html = readFileSync(page, 'utf8')
    // An installer is never linked past the count: straight to GitHub's file.
    assert.doesNotMatch(html, /href="https:\/\/github\.com\/YashIngole\/agent-moshpit\/releases\/latest\/download\//, page)
    for (const [, file] of html.matchAll(/href="\/get\/([^"]+)"/g)) {
      assert.ok(INSTALLERS.includes(file), `${page}: ${file}`)
      linked.add(file)
    }
  }
  assert.deepEqual([...linked].sort(), [...INSTALLERS].sort(), 'the home page offers every installer')
  const config = readFileSync(path.join(repo, 'tools/cloudflare/site.jsonc'), 'utf8')
  assert.match(config, /"run_worker_first": \["\/get\/\*", "\/install\.sh", "\/install\.ps1"\]/)
  assert.match(config, /"binding": "DB"/)
  // The names are the ones the install scripts take the same files by.
  const scripts = readFileSync(path.join(repo, 'site/install.sh'), 'utf8') + readFileSync(path.join(repo, 'site/install.ps1'), 'utf8')
  for (const file of INSTALLERS.filter(name => !/\.(msi|dmg)$/.test(name))) assert.ok(scripts.includes(file), file)
})
