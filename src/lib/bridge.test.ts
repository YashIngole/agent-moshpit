import { afterEach, expect, test, vi } from 'vitest'

const events = vi.hoisted(() => ({ listen: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: events.listen }))

afterEach(() => { vi.unstubAllGlobals(); vi.resetModules(); events.listen.mockReset() })

test('the pending tray request is consumed after the listener is registered', async () => {
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} })
  let registered!: (stop: () => void) => void
  events.listen.mockImplementation(() => new Promise(resolve => { registered = resolve }))
  const { bridge } = await import('./bridge')
  const consume = vi.fn()
  const unlisten = vi.fn()
  const stop = bridge.onNewAgent(consume)
  expect(consume).not.toHaveBeenCalled()
  registered(unlisten)
  await Promise.resolve()
  expect(consume).toHaveBeenCalledOnce()
  stop()
  expect(unlisten).toHaveBeenCalledOnce()
})

test('a closed window never consumes a pending tray request', async () => {
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} })
  let registered!: (stop: () => void) => void
  events.listen.mockImplementation(() => new Promise(resolve => { registered = resolve }))
  const { bridge } = await import('./bridge')
  const consume = vi.fn()
  const unlisten = vi.fn()
  bridge.onNewAgent(consume)()
  registered(unlisten)
  await Promise.resolve()
  expect(consume).not.toHaveBeenCalled()
  expect(unlisten).toHaveBeenCalledOnce()
})
