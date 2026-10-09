// A stand-in for an agent's program, for tests: it behaves in a terminal the way
// the real ones do, without a model behind it.
//
//   node fake-agent.mjs [task]
//
// It says it is ready; with a task it "works" for a moment and announces the end
// the way Codex does (a terminal notice). After that it reads lines:
//
//   ask    rings the bell and waits, as a program does when it wants an answer
//   work   prints for two seconds
//   path   names a file and a line, as a program does
//   exit   ends
//   else   is said back
import { createInterface } from 'node:readline'

const args = process.argv.slice(2)
const trustExcept = args.find(arg => arg.startsWith('--trust-except='))?.slice('--trust-except='.length)
let trustPending = trustExcept !== undefined && process.cwd().toLowerCase() !== trustExcept.toLowerCase()
const task = args.filter(arg => !arg.startsWith('--trust-except=')).join(' ')
const say = text => process.stdout.write(`${text}\r\n`)
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

async function work(label) {
  for (let i = 1; i <= 8; i++) {
    say(`${label} ${i}/8`)
    await sleep(250)
  }
  // A desktop notice, as a terminal program sends one.
  process.stdout.write('\x1b]9;Agent turn complete\x07')
  say('FINISHED')
}

async function ready() {
  say(`FAKE AGENT READY in ${process.cwd()} at ${process.stdout.columns}x${process.stdout.rows}`)
  if (task) { say(`TASK: ${task}`); await work('working') }
}
if (trustPending) say('Do you trust this folder?\r\n1. Yes, I trust this folder\r\n2. No, exit')
else await ready()

const lines = createInterface({ input: process.stdin })
for await (const line of lines) {
  if (trustPending) {
    trustPending = false
    process.stdout.write('\x1b[2J\x1b[H')
    await ready()
    continue
  }
  const said = line.trim()
  if (said === 'exit') break
  if (said === 'ask') {
    say('MAY I? (answer anything)')
    process.stdout.write('\x07')
  } else if (said === 'work') {
    await work('working')
  } else if (said === 'size') {
    say(`SIZE ${process.stdout.columns}x${process.stdout.rows}`)
  } else if (said === 'path') {
    // A file and a line, as a program names one: Ctrl and a click opens it in the editor.
    say('EDITED src/app.ts:42')
  } else if (said) {
    say(`YOU SAID: ${said}`)
  }
}
say('GOODBYE')
process.exit(0)
