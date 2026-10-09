// Read the Claude SDK initialization catalog without sending a user message.
// This is a live compatibility check, not an inference request.
import { spawn, execFileSync } from 'node:child_process'

const child = spawn(process.argv[2] || 'claude', [
  '--print', '--input-format', 'stream-json', '--output-format', 'stream-json',
  '--verbose', '--no-session-persistence', '--safe-mode'
], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
let pending = ''
let finished = false
const stop = () => {
  if (finished) return
  finished = true
  clearTimeout(timeout)
  if (process.platform === 'win32') {
    try { execFileSync('taskkill', ['/pid', String(child.pid), '/t', '/f'], { stdio: 'ignore', windowsHide: true }) } catch {}
  } else child.kill()
}
const timeout = setTimeout(() => { console.error('Catalog initialization timed out'); process.exitCode = 1; stop() }, 20000)
child.on('error', error => { console.error(error.message); process.exitCode = 1; stop() })
child.stdout.on('data', bytes => {
  pending += bytes.toString()
  while (pending.includes('\n')) {
    const end = pending.indexOf('\n')
    const line = pending.slice(0, end)
    pending = pending.slice(end + 1)
    try {
      const message = JSON.parse(line)
      if (message.type === 'control_response') {
        const reply = message.response?.response
        console.log(JSON.stringify({ subtype: message.response?.subtype, keys: Object.keys(reply || {}), models: reply?.models }, null, 2))
        stop()
      }
    } catch {}
  }
})
child.stderr.on('data', () => {})
child.on('exit', code => { if (!finished) { console.error(`CLI exited before returning a catalog (${code})`); process.exitCode = 1; stop() } })
child.stdin.write(JSON.stringify({ type: 'control_request', request_id: 'model-catalog', request: { subtype: 'initialize' } }) + '\n')
