// A stand-in installer, for tests: it puts the stand-in agent program in a folder,
// as npm would put a real one, and says what it is doing as it goes.
//
//   node install-late.mjs <folder>
import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const into = process.argv[2]
const here = path.dirname(fileURLToPath(import.meta.url))
const say = text => process.stdout.write(`${text}\r\n`)
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

say('INSTALLING LATE AGENT')
await sleep(600)
mkdirSync(into, { recursive: true })
copyFileSync(path.join(here, 'fake-agent.mjs'), path.join(into, 'fake-agent.mjs'))
// The same shape as the script npm writes for a program on Windows: the office reads
// which file it runs and starts node with that file directly.
const shim = ['@ECHO off', String.raw`"%_prog%"  "%dp0%\fake-agent.mjs" %*`, ''].join('\r\n')
writeFileSync(path.join(into, 'late.cmd'), shim)
await sleep(300)
say('added 1 package')
process.exit(0)
