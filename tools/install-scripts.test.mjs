// The install commands (site/install.sh, site/install.ps1) take an installer only from a
// release whose list of hashes is signed with the pinned key. Here both run against a local
// stand-in for GitHub's release addresses, signed with a throwaway key: a good release is
// installed, and a changed installer, a changed list, a missing signature, another key's
// signature or an installer the list does not name installs nothing.
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash, randomBytes } from 'node:crypto'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import os from 'node:os'
import path from 'node:path'
import test, { after, before } from 'node:test'
import { fileURLToPath } from 'node:url'
import { checksumText } from './release-checksums.mjs'

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const TAG = 'v9.9.9'
const SIGNER = 'installers@agentmoshpit.com'
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

/** A POSIX shell, and on Windows the one Git brings with ssh-keygen, tar and curl. */
function posixShell() {
  if (process.platform !== 'win32') return '/bin/sh'
  for (const candidate of ['sh', 'C:\\Program Files\\Git\\bin\\sh.exe']) {
    if (spawnSync(candidate, ['-c', 'command -v ssh-keygen'], { encoding: 'utf8' }).status === 0) return candidate
  }
  throw new Error('A POSIX shell with ssh-keygen is needed (Git for Windows has one).')
}
const SH = posixShell()
const sh = (script, ...args) => {
  const run = spawnSync(SH, ['-c', script, 'sh', ...args], { encoding: 'utf8' })
  assert.equal(run.status, 0, run.stderr)
  return run.stdout
}
/** Run a program without blocking: the stand-in server answers from this same process. */
const run = (command, args, options) => new Promise(resolve => {
  const child = spawn(command, args, { ...options, stdio: ['ignore', 'pipe', 'pipe'] })
  let stdout = '', stderr = ''
  child.stdout.on('data', chunk => (stdout += chunk))
  child.stderr.on('data', chunk => (stderr += chunk))
  child.on('close', status => resolve({ status, stdout, stderr }))
})
/** A path as the shell names it: Git's shell on Windows wants /c/... */
const posix = p => (process.platform === 'win32' ? sh('cygpath -u "$1"', p).trim() : p)

let dir, server, port
const files = {}
const release = { list: '', sig: null, served: {} }

before(async () => {
  dir = mkdtempSync(path.join(os.tmpdir(), 'moshpit-install-test-'))
  for (const key of ['good', 'other']) sh('ssh-keygen -q -t ed25519 -N "" -C test -f "$1"', posix(path.join(dir, key)))
  files['agent-moshpit_linux_amd64.deb'] = randomBytes(4096)
  files['agent-moshpit_windows_x64-setup.exe'] = randomBytes(4096)
  mkdirSync(path.join(dir, 'app', 'Agent Moshpit.app', 'Contents'), { recursive: true })
  writeFileSync(path.join(dir, 'app', 'Agent Moshpit.app', 'Contents', 'Info.plist'), 'stand-in')
  sh('tar -czf "$1" -C "$2" "Agent Moshpit.app"', posix(path.join(dir, 'app.tar.gz')), posix(path.join(dir, 'app')))
  files['agent-moshpit_darwin_aarch64.app.tar.gz'] = readFileSync(path.join(dir, 'app.tar.gz'))
  server = createServer((request, response) => {
    const name = decodeURIComponent(request.url.split('/').pop())
    const latest = request.url.startsWith('/releases/latest/download/')
    const pinned = request.url.startsWith(`/releases/download/${TAG}/`)
    const body = latest && name === 'SHA256SUMS' ? release.list : latest && name === 'SHA256SUMS.sig' ? release.sig : pinned ? release.served[name] : null
    if (body == null) { response.writeHead(404).end(); return }
    response.writeHead(200).end(body)
  })
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
  port = server.address().port
})
after(() => {
  server?.close()
  if (dir) rmSync(dir, { recursive: true, force: true })
})

/** Publish a stand-in release: its list, signed by `key`, and the files it serves. */
function publish({ key = 'good', listed = files, served = files, list } = {}) {
  release.list = list ?? checksumText(TAG, Object.entries(listed).map(([name, bytes]) => ({ name, sha256: sha256(bytes) })))
  writeFileSync(path.join(dir, 'SHA256SUMS'), release.list)
  rmSync(path.join(dir, 'SHA256SUMS.sig'), { force: true })
  sh('ssh-keygen -q -Y sign -f "$1" -n agentmoshpit-install "$2"', posix(path.join(dir, key)), posix(path.join(dir, 'SHA256SUMS')))
  release.sig = readFileSync(path.join(dir, 'SHA256SUMS.sig'))
  release.served = served
}
const signers = () => `${SIGNER} namespaces="agentmoshpit-install" ${readFileSync(path.join(dir, 'good.pub'), 'utf8').split(' ').slice(0, 2).join(' ')}`
const changed = bytes => { const copy = Buffer.from(bytes); copy[100] ^= 1; return copy }

/** The script as published, pointed at the stand-in and trusting the throwaway key. */
function prepared(file, replacements) {
  let text = readFileSync(path.join(repo, 'site', file), 'utf8')
  for (const [pattern, line] of replacements) {
    assert.match(text, pattern, `${file} no longer has ${pattern}`)
    text = text.replace(pattern, () => line)
  }
  const copy = path.join(dir, `copy-${file}`)
  writeFileSync(copy, text)
  return copy
}

async function runSh(system, machine, onlyFetch = false) {
  const fake = path.join(dir, 'bin')
  mkdirSync(fake, { recursive: true })
  const tool = (name, body) => writeFileSync(path.join(fake, name), `#!/bin/sh\n${body}\n`, { mode: 0o755 })
  tool('uname', 'case "$1" in -m) echo "$MOCK_MACHINE" ;; *) echo "$MOCK_SYSTEM" ;; esac')
  tool('sudo', 'exec "$@"')
  tool('apt-get', 'printf "%s\\n" "$*" >>"$MOCK_LOG"; cp "$3" "$MOCK_LOG.installed"')
  tool('pgrep', 'exit 1')
  tool('open', 'printf "open %s\\n" "$*" >>"$MOCK_LOG"')
  const log = path.join(dir, 'installs.log')
  rmSync(log, { force: true })
  rmSync(`${log}.installed`, { force: true })
  const script = prepared('install.sh', [
    [/^set -eu$/m, `set -eu\nPATH="${posix(fake)}:$PATH"`],
    [/^REPO=.*$/m, `REPO="http://127.0.0.1:${port}/releases"`],
    [/^SIGNERS=.*$/m, `SIGNERS='${signers()}'`],
    // The stand-in speaks plain HTTP on this computer; the real one is HTTPS only.
    [/^download\(\) \{.*$/m, 'download() { curl -fL "$@"; }']
  ])
  const result = await run(SH, [posix(script)], {
    env: { ...process.env, HOME: dir, MOCK_SYSTEM: system, MOCK_MACHINE: machine, MOCK_LOG: posix(log), MOSHPIT_INSTALL_ONLY_FETCH: onlyFetch ? '1' : '', DISPLAY: '', WAYLAND_DISPLAY: '' }
  })
  return { ...result, installed: existsSync(`${log}.installed`) ? readFileSync(`${log}.installed`) : null }
}

test('the pinned key is the same in both install commands and the repository', () => {
  const pinned = readFileSync(path.join(repo, 'tools', 'install-signers'), 'utf8').trim()
  assert.match(pinned, /^installers@agentmoshpit\.com namespaces="agentmoshpit-install" ssh-ed25519 [A-Za-z0-9+/=]+$/)
  assert.ok(readFileSync(path.join(repo, 'site', 'install.sh'), 'utf8').includes(`SIGNERS='${pinned}'`))
  assert.ok(readFileSync(path.join(repo, 'site', 'install.ps1'), 'utf8').includes(`$signers = '${pinned}'`))
})

test('install.sh installs only what the signed list names', async () => {
  const deb = 'agent-moshpit_linux_amd64.deb'
  publish()
  const good = await runSh('Linux', 'x86_64')
  assert.equal(good.status, 0, good.stderr)
  assert.deepEqual(good.installed, files[deb], 'apt was handed the checked file')

  const refused = async (what, pattern) => {
    const attempt = await runSh('Linux', 'x86_64')
    assert.notEqual(attempt.status, 0, `${what} was installed`)
    assert.equal(attempt.installed, null, `${what} reached apt`)
    assert.match(attempt.stderr, pattern, what)
  }
  publish({ served: { ...files, [deb]: changed(files[deb]) } })
  await refused('a changed installer', /is not the file the signed list names/)
  publish()
  release.list = release.list.replace(sha256(files[deb]), sha256(changed(files[deb])))
  release.served = { ...files, [deb]: changed(files[deb]) }
  await refused('a changed list', /not signed with Agent Moshpit's key/)
  publish()
  release.sig = null
  await refused('a missing signature', /signature of the release's list of installers could not be downloaded/)
  publish({ key: 'other' })
  await refused("another key's signature", /not signed with Agent Moshpit's key/)
  const { [deb]: _, ...others } = files
  publish({ listed: others })
  await refused('an installer the list does not name', /is not in the signed list of installers/)
})

test('install.sh checks the Mac download the same way', async () => {
  const app = 'agent-moshpit_darwin_aarch64.app.tar.gz'
  publish()
  const good = await runSh('Darwin', 'arm64', true)
  assert.equal(good.status, 0, good.stderr)
  assert.match(good.stdout, /Fetched and checked agent-moshpit_darwin_aarch64\.app\.tar\.gz \(v9\.9\.9\)/)
  publish({ served: { ...files, [app]: changed(files[app]) } })
  const bad = await runSh('Darwin', 'arm64', true)
  assert.notEqual(bad.status, 0)
  assert.match(bad.stderr, /is not the file the signed list names/)
})

/** Every PowerShell this computer has: Windows PowerShell 5.1, the one people run install.ps1
 * in on Windows, and PowerShell 7 where it is installed. Windows must have at least one. */
const powershells = ['powershell', 'pwsh'].filter(name => spawnSync(name, ['-NoProfile', '-Command', 'exit 0']).status === 0)
// Windows PowerShell started from PowerShell 7 (as on CI) would inherit 7's module path.
const plainEnv = Object.fromEntries(Object.entries(process.env).filter(([name]) => name.toLowerCase() !== 'psmodulepath'))

test('install.ps1 runs only what the signed list names', { skip: powershells.length === 0 && process.platform !== 'win32' ? 'no PowerShell on this computer' : false }, async () => {
  assert.ok(powershells.length > 0, 'Windows PowerShell was not found')
  const exe = 'agent-moshpit_windows_x64-setup.exe'
  const script = prepared('install.ps1', [
    [/^ {4}\$repo = .*$/m, `    $repo = 'http://127.0.0.1:${port}/releases'`],
    [/^ {4}\$signers = .*$/m, `    $signers = '${signers()}'`]
  ])
  for (const powershell of powershells) {
    const attempt = () => run(powershell, ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', script], { env: { ...plainEnv, MOSHPIT_INSTALL_ONLY_FETCH: '1' } })
    publish()
    const good = await attempt()
    assert.equal(good.status, 0, `${powershell}: ${good.stderr}${good.stdout}`)
    assert.ok(good.stdout.includes(`Fetched and checked ${exe} (v9.9.9, 4096 bytes)`), `${powershell}: ${good.stdout}`)
    const refused = async (what, pattern) => {
      const bad = await attempt()
      assert.notEqual(bad.status, 0, `${powershell}: ${what} was accepted`)
      assert.match(bad.stderr + bad.stdout, pattern, `${powershell}: ${what}`)
    }
    publish({ served: { ...files, [exe]: changed(files[exe]) } })
    await refused('a changed installer', /is not the file the signed list names/)
    publish()
    release.sig = null
    await refused('a missing signature', /signature of the release's list of installers could not be downloaded/)
    publish({ key: 'other' })
    await refused("another key's signature", /not signed with Agent Moshpit's key/)
  }
})
