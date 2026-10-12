// Deterministic stand-in for Codex's terminal identity and rollout persistence.
//
// Its terminal's title is as Codex writes it when asked for its session's id and its
// thread's title: the id cut short, a bar, then the title. Until the thread has a title
// that second part is the id again, and while it works a mark that turns follows each.
import { randomUUID } from 'node:crypto'
import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'

const args = process.argv.slice(2)
let session = args.includes('--resume') ? args[args.indexOf('--resume') + 1] : randomUUID()
let thread = ''
const title = () => process.stdout.write(`\x1b]2;${session.slice(0, 27)}...${thread ? '' : ' ⠧'} | ${thread || `${session} ⠧`}\x07`)
function announce() {
  const folder = path.join(process.env.CODEX_HOME, 'sessions', '2026', '10', '09')
  mkdirSync(folder, { recursive: true })
  writeFileSync(path.join(folder, `rollout-${session}.jsonl`), JSON.stringify({ type: 'session_meta', payload: { id: session, cwd: process.cwd() } }) + '\n')
  thread = ''
  title()
  process.stdout.write(`SESSION ${session}\r\n`)
}
announce()
for await (const line of createInterface({ input: process.stdin })) {
  const said = line.trim()
  if (said === '/new') { session = randomUUID(); announce() }
  // The first thing it is asked becomes what its thread is called.
  else if (said && !thread) { thread = said; title() }
}
