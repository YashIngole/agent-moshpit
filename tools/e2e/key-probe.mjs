// A stand-in for an agent's program that shows every byte it is sent, for checking
// which keys reach a program in a terminal and which the window keeps.
//
//   node key-probe.mjs
//
// Each piece that arrives is printed as `KEY <hex>`. Ctrl+C ends it.
process.stdin.setRawMode?.(true)
process.stdin.resume()
process.stdout.write('KEY PROBE READY\r\n')
process.stdin.on('data', chunk => {
  if (chunk.length === 1 && chunk[0] === 3) process.exit(0)
  process.stdout.write(`KEY ${chunk.toString('hex')}\r\n`)
})
