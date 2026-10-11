// A pretend Deepgram for the desktop voice test: a WebSocket server on this computer that
// answers the app's audio the way /v1/listen does. The app is pointed at it with
// MOSHPIT_VOICE_ENDPOINT (debug builds, named instances only). Nothing leaves this computer.
//
// `mode` decides how it behaves: 'ok', 'reject' (401 before the upgrade, as Deepgram answers a
// bad key) or 'drop' (the connection is cut after the first settled words).
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'

const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11'

function frame(opcode, payload) {
  const body = Buffer.from(payload)
  const head = body.length < 126 ? Buffer.from([0x80 | opcode, body.length])
    : body.length < 65536 ? Buffer.from([0x80 | opcode, 126, body.length >> 8, body.length & 255])
    : (() => { const h = Buffer.alloc(10); h[0] = 0x80 | opcode; h[1] = 127; h.writeBigUInt64BE(BigInt(body.length), 2); return h })()
  return Buffer.concat([head, body])
}

/** Client frames from the buffer, and what is left of it. Client frames are always masked. */
function frames(buffer) {
  const out = []
  for (;;) {
    if (buffer.length < 2) break
    const opcode = buffer[0] & 15
    let length = buffer[1] & 127
    let at = 2
    if (length === 126) { if (buffer.length < 4) break; length = buffer.readUInt16BE(2); at = 4 }
    else if (length === 127) { if (buffer.length < 10) break; length = Number(buffer.readBigUInt64BE(2)); at = 10 }
    if (buffer.length < at + 4 + length) break
    const mask = buffer.subarray(at, at + 4)
    const payload = Buffer.from(buffer.subarray(at + 4, at + 4 + length))
    for (let i = 0; i < payload.length; i++) payload[i] ^= mask[i % 4]
    out.push({ opcode, payload })
    buffer = buffer.subarray(at + 4 + length)
  }
  return { out, rest: buffer }
}

const results = (transcript, final, extra = {}) => JSON.stringify({ type: 'Results', is_final: final, speech_final: final, channel: { alternatives: [{ transcript, confidence: 0.98 }] }, ...extra })

export async function startMock(port) {
  const mock = { mode: 'ok', sessions: [] }
  const server = createServer((_, response) => { response.writeHead(426); response.end() })
  const sockets = new Set()
  server.on('upgrade', (request, socket) => {
    sockets.add(socket)
    const session = { url: request.url, authorization: request.headers.authorization, samples: 0, finals: 0, messages: [], closed: false }
    mock.sessions.push(session)
    if (mock.mode === 'reject') {
      socket.end('HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}')
      return
    }
    const accept = createHash('sha1').update(request.headers['sec-websocket-key'] + GUID).digest('base64')
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`)
    const say = text => { if (!socket.destroyed) socket.write(frame(1, text)) }
    let pending = Buffer.alloc(0)
    socket.on('data', chunk => {
      const { out, rest } = frames(Buffer.concat([pending, chunk]))
      pending = rest
      for (const { opcode, payload } of out) {
        if (opcode === 2) {
          const before = session.samples
          session.samples += payload.length / 2
          // Live words every half second of audio; settled ones every second and a half.
          if (Math.floor(session.samples / 8000) > Math.floor(before / 8000)) say(results(`mock phrase ${session.finals + 1}`, false))
          if (Math.floor(session.samples / 24000) > Math.floor(before / 24000)) {
            session.finals += 1
            say(results(`Mock phrase ${session.finals}.`, true))
            if (mock.mode === 'drop') { socket.destroy(); session.closed = true }
          }
        } else if (opcode === 1) {
          const message = JSON.parse(payload.toString())
          session.messages.push(message.type)
          if (message.type === 'Finalize') say(results('Last words.', true, { from_finalize: true }))
          if (message.type === 'CloseStream') {
            say(JSON.stringify({ type: 'Metadata', request_id: 'mock', duration: session.samples / 16000 }))
            socket.end(frame(8, Buffer.from([0x03, 0xe8])))
            session.closed = true
          }
        } else if (opcode === 8) {
          session.closed = true
          socket.end(frame(8, Buffer.from([0x03, 0xe8])))
        }
      }
    })
    socket.on('error', () => {})
    socket.on('close', () => { session.closed = true })
  })
  await new Promise(resolve => server.listen(port, '127.0.0.1', resolve))
  mock.url = `ws://127.0.0.1:${port}`
  mock.close = () => new Promise(resolve => { for (const socket of sockets) socket.destroy(); server.close(resolve) })
  return mock
}
