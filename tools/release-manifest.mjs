// Writes latest.json, the file a running office reads to learn that a newer version
// is out (see src-tauri/src/update.rs), and attaches it to a release.
//
//   node tools/release-manifest.mjs <owner/repo> <release id> <tag>
//
// Run by .github/workflows/release.yml once every installer is attached. Each file an
// office can update itself from was signed as it was built, and its signature attached
// beside it as <file>.sig. This reads those, names each one the way the updater asks for
// it ("windows-x86_64-nsis", "darwin-aarch64", ...), and keeps the .sig inputs for safe retries:
// they are also inside latest.json. Needs the `gh` command and a token in GH_TOKEN.
import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

/** The updater's names for a file: by system, processor and kind of installer; the plainest name last. */
export function names(file) {
  const system = /_windows_/.test(file) ? 'windows' : /_darwin_/.test(file) ? 'darwin' : /_linux_/.test(file) ? 'linux' : null
  const processor = /(aarch64|arm64)/.test(file) ? 'aarch64' : /(x64|amd64|x86_64)/.test(file) ? 'x86_64' : null
  const kinds = [
    [/-setup\.exe$/, 'nsis', true],
    [/\.msi$/, 'msi', false],
    [/\.app\.tar\.gz$/, 'app', true],
    [/\.AppImage$/, 'appimage', true],
    [/\.deb$/, 'deb', false],
    [/\.rpm$/, 'rpm', false]
  ]
  const kind = kinds.find(([ending]) => ending.test(file))
  if (!system || !processor || !kind) return []
  const [, installer, usual] = kind
  // An office that does not know how it was installed asks by system and processor alone.
  return usual ? [`${system}-${processor}-${installer}`, `${system}-${processor}`] : [`${system}-${processor}-${installer}`]
}

export function buildManifest(repo, tag, assets, readSignature, date = new Date().toISOString()) {
  const platforms = {}
  const signatures = assets.filter(asset => asset.name.endsWith('.sig'))
  for (const signature of signatures) {
    const file = signature.name.slice(0, -'.sig'.length)
    if (!assets.some(asset => asset.name === file)) continue
    const entry = {
      signature: readSignature(signature).trim(),
      url: `https://github.com/${repo}/releases/download/${tag}/${file}`
    }
    for (const name of names(file)) platforms[name] = entry
  }

  const needed = ['windows-x86_64', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64']
  const missing = needed.filter(name => !platforms[name])
  if (missing.length > 0) {
    throw new Error(`No signed update file for: ${missing.join(', ')}. Nothing was attached.`)
  }

  return {
    version: tag.replace(/^v/, ''),
    notes: `What is new: https://github.com/${repo}/releases/tag/${tag}`,
    pub_date: date,
    platforms
  }
}

export function main() {
  const [repo, release, tag] = process.argv.slice(2)
  if (!repo || !release || !tag) {
    console.error('usage: node tools/release-manifest.mjs <owner/repo> <release id> <tag>')
    process.exit(2)
  }
  const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
  const assets = JSON.parse(gh('api', `repos/${repo}/releases/${release}/assets?per_page=100`))

  const manifest = buildManifest(repo, tag, assets, signature => gh('api', '-H', 'Accept: application/octet-stream', `repos/${repo}/releases/assets/${signature.id}`))
  writeFileSync('latest.json', `${JSON.stringify(manifest, null, 2)}\n`)

  // One latest.json, the one just written.
  for (const old of assets.filter(asset => asset.name === 'latest.json')) gh('api', '-X', 'DELETE', `repos/${repo}/releases/assets/${old.id}`)
  gh('api', '-X', 'POST', '-H', 'Content-Type: application/json', `https://uploads.github.com/repos/${repo}/releases/${release}/assets?name=latest.json`, '--input', 'latest.json')

  console.log(`latest.json for ${manifest.version}:`)
  for (const [name, entry] of Object.entries(manifest.platforms)) console.log(`  ${name.padEnd(24)} ${entry.url.split('/').pop()}`)

}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()
