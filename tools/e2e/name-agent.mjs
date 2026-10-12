// Deterministic stand-in for how Claude Code names a conversation, and what it puts in
// its terminal's title.
//
// Claude Code keeps a file about each running session, with a name in it. Nobody having
// named the conversation, the name is one it makes up from the folder and two characters,
// and says so ("derived"); `/rename` makes it the user's ("user"). Separately, once it has
// been given something to do it titles its terminal with a few words about that, behind a
// mark that shows whether it is at work.
import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'

const folder = path.join(process.env.CLAUDE_CONFIG_DIR, 'sessions')
mkdirSync(folder, { recursive: true })
const called = (name, nameSource) =>
  writeFileSync(path.join(folder, `${process.pid}.json`), JSON.stringify({ pid: process.pid, sessionId: 'name-agent-session', cwd: process.cwd(), name, nameSource, nameSince: Date.now(), status: 'idle' }))
const title = text => process.stdout.write(`\x1b]0;${text}\x07`)

called(`${path.basename(process.cwd()).toLowerCase()}-b3`, 'derived')
title('✳ Name Agent')
process.stdout.write('NAME AGENT READY\r\n')
for await (const line of createInterface({ input: process.stdin })) {
  const said = line.trim()
  if (said.startsWith('/rename ')) called(said.slice('/rename '.length), 'user')
  else if (said) {
    title(`◐ ${said}`)
    setTimeout(() => title(`✳ ${said}`), 200)
  }
}
