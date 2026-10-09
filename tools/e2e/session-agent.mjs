// Deterministic stand-in for Codex's terminal identity and rollout persistence.
import { randomUUID } from 'node:crypto'
import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'

const args = process.argv.slice(2)
let session = args.includes('--resume') ? args[args.indexOf('--resume') + 1] : randomUUID()
function announce() {
  const folder = path.join(process.env.CODEX_HOME, 'sessions', '2026', '10', '09')
  mkdirSync(folder, { recursive: true })
  writeFileSync(path.join(folder, `rollout-${session}.jsonl`), JSON.stringify({ type: 'session_meta', payload: { id: session, cwd: process.cwd() } }) + '\n')
  process.stdout.write(`\x1b]2;${session.slice(0, 27)}...\x07SESSION ${session}\r\n`)
}
announce()
for await (const line of createInterface({ input: process.stdin })) {
  if (line.trim() === '/new') { session = randomUUID(); announce() }
}
