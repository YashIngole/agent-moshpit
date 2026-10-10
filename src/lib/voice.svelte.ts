import { bridge } from './bridge'
import { DEFAULT_VOICE, EMPTY_VOICE, type VoiceInputs, type VoiceModel, type VoiceSettings, type VoiceView } from './voice'
import { terms } from './terms'
import { tick } from 'svelte'

class Voice {
  settings = $state<VoiceSettings>({ ...DEFAULT_VOICE })
  view = $state<VoiceView>({ ...EMPTY_VOICE })
  problem = $state('')
  changing = $state(false)
  acting = $state(false)
  inputs = $state<VoiceInputs>({ devices: [], default: null })
  inputsLoading = $state(false)
  inputsProblem = $state('')
  panel = false
  active = $derived(['preparing', 'listening', 'transcribing'].includes(this.view.phase))
  ready = $derived(this.view.models.some(m => m.id === this.settings.model && m.ready))
  #polling = false
  #alive = false
  #request = 0
  #restoreFocus = false

  start() {
    this.#alive = true
    void bridge.settings().then(s => { if (this.#alive) this.settings = { ...DEFAULT_VOICE, ...s.voice } }).catch(() => {
      if (this.#alive) this.problem = 'Voice settings could not load. Reopen the window to try again.'
    })
    void this.refresh()
    const poll = setInterval(() => {
      if (this.panel || this.active || this.view.busy || this.view.verifying || this.view.downloading) void this.refresh()
    }, 200)
    const hide = () => { if (document.hidden) this.cancel() }
    const leave = () => this.cancel()
    const focus = (event: FocusEvent) => {
      if (!(event.target instanceof Element) || !event.target.closest('.voice-status, .mic')) this.#restoreFocus = false
    }
    document.addEventListener('visibilitychange', hide)
    window.addEventListener('pagehide', leave)
    document.addEventListener('focusin', focus)
    return () => {
      this.#alive = false
      this.#request += 1
      clearInterval(poll)
      this.cancel()
      document.removeEventListener('visibilitychange', hide)
      window.removeEventListener('pagehide', leave)
      document.removeEventListener('focusin', focus)
    }
  }
  async refresh() {
    if (this.#polling) return
    this.#polling = true
    const request = this.#request
    try {
      const view = await bridge.voiceView()
      if (this.#alive && request === this.#request && view.sequence >= this.view.sequence) {
        const inserted = this.view.phase === 'transcribing' && view.phase === 'idle' && !!view.message
        this.view = view
        // Stop is a button; return its keyboard to the same terminal after insertion.
        // A user who moved elsewhere while waiting keeps their focus.
        if (inserted && view.agent && (this.#restoreFocus || document.activeElement?.closest('.voice-status, .mic'))) {
          const target = view.agent
          await tick()
          if (this.#alive && request === this.#request && (this.#restoreFocus || document.activeElement?.closest('.voice-status, .mic'))) terms.get(target)?.focus()
          this.#restoreFocus = false
        }
      }
    } catch { /* A destroyed window has nothing to update. Actions surface failures. */ }
    finally { this.#polling = false }
  }
  async act(action: () => Promise<unknown>) {
    this.problem = ''
    try { await action() }
    catch (error) { this.problem = typeof error === 'string' ? error : error instanceof Error ? error.message : 'Voice input could not continue. Try again.' }
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
    if (this.acting || this.changing) return
    if (this.view.phase !== 'listening' && (this.active || this.view.busy)) return
    // The Stop button disappears while transcribing. Remember its focus before
    // the browser hands focus to the body, and revoke this if the user moves on.
    if (this.view.phase === 'listening') this.#restoreFocus = !!document.activeElement?.closest('.voice-status, .mic')
    this.acting = true
    try {
      await this.act(() => this.view.phase === 'listening' ? bridge.voiceStop() : bridge.voiceStart(agent))
    } finally { this.acting = false }
  }
  async loadInputs() {
    if (this.inputsLoading) return
    this.inputsLoading = true
    this.inputsProblem = ''
    try { this.inputs = await bridge.voiceInputs() }
    catch (error) { this.inputsProblem = typeof error === 'string' ? error : 'Microphones could not be listed. Reconnect your microphone and refresh.' }
    finally { this.inputsLoading = false }
  }
  cancel() {
    this.#restoreFocus = false
    this.#request += 1
    this.view = { ...this.view, phase: 'idle', agent: null, started_ms: null, message: '', level: 0, transcribing_ms: null }
    void bridge.voiceCancel().then(() => this.refresh()).catch(() => {
      if (this.#alive) this.problem = 'Voice cancellation could not be confirmed. Close the window to stop capture.'
    })
  }
  download(model: VoiceModel) { return this.act(() => bridge.voiceDownload(model)) }
  remove(model: VoiceModel) { return this.act(() => bridge.voiceRemove(model)) }
  cancelDownload() { return this.act(() => bridge.voiceCancelDownload()) }
}
export const voice = new Voice()
