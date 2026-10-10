// Checks, before anything is built, that a version tag is one this checkout can ship.
//
//   node tools/release-check.mjs <tag>
//
// latest.json offers the tag's version (tools/release-manifest.mjs). The app it installs
// reports the version in its own files, so all of them must say the tag's version: an
// office that updates to a build calling itself older would be offered it again forever.
import { readFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

/** What is wrong with shipping this tag from these files. Empty when nothing is. */
export function versionProblems(tag, { packageJson, cargoToml, tauriConf }) {
  if (!/^v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(tag)) return [`${tag} is not a version tag such as v1.2.3.`]
  const wanted = tag.slice(1)
  const found = {
    'package.json': JSON.parse(packageJson).version,
    'src-tauri/Cargo.toml': cargoToml.match(/^\[package\][^[]*?^version\s*=\s*"([^"]+)"/m)?.[1],
    'src-tauri/tauri.conf.json': JSON.parse(tauriConf).version
  }
  return Object.entries(found)
    .filter(([, version]) => version !== wanted)
    .map(([file, version]) => `${file} says ${version ?? 'no version'}, not ${wanted}.`)
}

export function main() {
  const tag = process.argv[2]
  if (!tag) {
    console.error('usage: node tools/release-check.mjs <tag>')
    process.exit(2)
  }
  const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
  const problems = versionProblems(tag, { packageJson: read('package.json'), cargoToml: read('src-tauri/Cargo.toml'), tauriConf: read('src-tauri/tauri.conf.json') })
  if (problems.length > 0) {
    for (const problem of problems) console.error(`::error::${problem}`)
    process.exit(1)
  }
  console.log(`${tag}: package.json, Cargo.toml and tauri.conf.json agree.`)
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()
