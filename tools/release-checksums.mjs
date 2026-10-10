// Writes SHA256SUMS, the signed list the install commands trust, and attaches it to a release.
//
//   node tools/release-checksums.mjs write <owner/repo> <release id> <tag>
//   node tools/release-checksums.mjs upload <owner/repo> <release id>
//
// Run by .github/workflows/release.yml once every installer is attached. `write` fetches
// each installer from the draft and lists its SHA-256 under a first line naming the release.
// The workflow then signs the list with the installer key (`ssh-keygen -Y sign`), checks the
// signature against tools/install-signers, and `upload` attaches SHA256SUMS and its .sig.
// site/install.sh and site/install.ps1 pin that key: they take the release named in a list
// whose signature matches it, and an installer only when its hash is the one listed.
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

export const LIST = 'SHA256SUMS'
const HEADER = /^# Agent Moshpit (v\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)$/

/** Every file a person may install from a release: not signatures, manifests or the list itself. */
export function installers(assets) {
  return assets.filter(asset => !asset.name.endsWith('.sig') && asset.name !== 'latest.json' && asset.name !== LIST)
}

export function checksumText(tag, entries) {
  if (!HEADER.test(`# Agent Moshpit ${tag}`)) throw new Error(`${tag} is not a version tag.`)
  if (entries.length === 0) throw new Error('A release with no installers has nothing to list.')
  const lines = [...entries]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(({ name, sha256 }) => {
      if (!/^[0-9a-f]{64}$/.test(sha256) || !/^[\w.-]+$/.test(name)) throw new Error(`Bad entry: ${name}`)
      return `${sha256}  ${name}`
    })
  return `# Agent Moshpit ${tag}\n${lines.join('\n')}\n`
}

/** The release a list names, and each file's hash. What the install commands read, in JavaScript. */
export function parseChecksums(text) {
  const [first, ...rest] = text.split('\n')
  const tag = first.match(HEADER)?.[1]
  if (!tag) throw new Error('The list names no release.')
  const files = new Map()
  for (const line of rest.filter(Boolean)) {
    const match = line.match(/^([0-9a-f]{64}) {2}([\w.-]+)$/)
    if (!match) throw new Error(`Unreadable line: ${line}`)
    files.set(match[2], match[1])
  }
  return { tag, files }
}

export function main() {
  const [command, repo, release, tag] = process.argv.slice(2)
  if (!['write', 'upload'].includes(command) || !repo || !release || (command === 'write' && !tag)) {
    console.error('usage: node tools/release-checksums.mjs write <owner/repo> <release id> <tag>\n       node tools/release-checksums.mjs upload <owner/repo> <release id>')
    process.exit(2)
  }
  const gh = (args, options = {}) => execFileSync('gh', args, { maxBuffer: 1024 * 1024 * 1024, ...options })
  const assets = JSON.parse(gh(['api', `repos/${repo}/releases/${release}/assets?per_page=100`], { encoding: 'utf8' }))
  if (command === 'write') {
    const entries = installers(assets).map(asset => {
      const bytes = gh(['api', '-H', 'Accept: application/octet-stream', `repos/${repo}/releases/assets/${asset.id}`])
      if (bytes.length !== asset.size) throw new Error(`${asset.name}: fetched ${bytes.length} bytes of ${asset.size}.`)
      return { name: asset.name, sha256: createHash('sha256').update(bytes).digest('hex') }
    })
    writeFileSync(LIST, checksumText(tag, entries))
    console.log(`${LIST} for ${tag}: ${entries.map(e => e.name).join(', ')}`)
    return
  }
  for (const old of assets.filter(asset => asset.name === LIST || asset.name === `${LIST}.sig`)) gh(['api', '-X', 'DELETE', `repos/${repo}/releases/assets/${old.id}`])
  for (const name of [LIST, `${LIST}.sig`]) {
    gh(['api', '-X', 'POST', '-H', 'Content-Type: text/plain', `https://uploads.github.com/repos/${repo}/releases/${release}/assets?name=${name}`, '--input', name], { stdio: ['ignore', 'ignore', 'inherit'] })
  }
  console.log(`Attached ${LIST} and ${LIST}.sig.`)
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()
