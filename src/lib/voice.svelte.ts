import { bridge } from './bridge'
import { DEFAULT_VOICE, EMPTY_VOICE, type VoiceModel, type VoiceSettings, type VoiceView } from './voice'

class Voice {
  settings = $state<VoiceSettings>({ ...DEFAULT_VOICE })
  view = $state<VoiceView>({ ...EMPTY_VOICE })
  problem = $state('')
  changing = $state(false)
  panel = false
  active = $derived(['preparing', 'listening', 'transcribing'].includes(this.view.phase))
  ready = $derived(this.view.models.some(m => m.id === this.settings.model && m.ready))
  #polling = false
  #alive = false
  #request = 0

  start() {
    this.#alive = true
    void bridge.settings().then(s => { if (this.#alive) this.settings = s.voice }).catch(() => {
      if (this.#alive) this.problem = 'Voice settings could not load. Reopen the window to try again.'
    })
    void this.refresh()
    const poll = setInterval(() => {
      if (this.panel || this.active || this.view.busy || this.view.verifying || this.view.downloading) void this.refresh()
    }, 200)
    const hide = () => { if (document.hidden) this.cancel() }
    const leave = () => this.cancel()
    document.addEventListener('visibilitychange', hide)
    window.addEventListener('pagehide', leave)
    return () => {
      this.#alive = false
      this.#request += 1
      clearInterval(poll)
      this.cancel()
      document.removeEventListener('visibilitychange', hide)
      window.removeEventListener('pagehide', leave)
    }
  }
  async refresh() {
    if (this.#polling) return
    this.#polling = true
    const request = this.#request
    try {
      const view = await bridge.voiceView()
      if (this.#alive && request === this.#request && view.sequence >= this.view.sequence) this.view = view
    } catch { /* A destroyed window has nothing to update. Actions surface failures. */ }
    finally { this.#polling = false }
  }
  async act(action: () => Promise<unknown>) {
    this.problem = ''
    try { await action() }
    catch (error) { this.problem = typeof error === 'string' ? error : 'Voice input could not continue. Try again.' }
    await this.refresh()
  }
  async configure(patch: Partial<VoiceSettings>) {
    if (this.changing) return
    this.changing = true
    this.cancel()
    const settings = { ...this.settings, ...patch }
    await this.act(async () => { await bridge.voiceConfig(settings); this.settings = settings })
    this.changing = false
  }
  async toggle(agent: string) {
    if (this.view.phase === 'listening') return this.act(() => bridge.voiceStop())
    if (this.active || this.view.busy || this.changing) return
    await this.act(() => bridge.voiceStart(agent))
  }
  cancel() {
    this.#request += 1
    this.view = { ...this.view, phase: 'idle', agent: null, started_ms: null, message: '' }
    void bridge.voiceCancel().then(() => this.refresh()).catch(() => {
      if (this.#alive) this.problem = 'Voice cancellation could not be confirmed. Close the window to stop capture.'
    })
  }
  download(model: VoiceModel) { return this.act(() => bridge.voiceDownload(model)) }
  remove(model: VoiceModel) { return this.act(() => bridge.voiceRemove(model)) }
  cancelDownload() { return this.act(() => bridge.voiceCancelDownload()) }
}
export const voice = new Voice()
