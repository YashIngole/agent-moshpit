import { bridge } from './bridge'
import { DEFAULT_VOICE, EMPTY_VOICE, HOLD_MS, keyProblem, type VoiceField, type VoiceInputs, type VoiceProvider, type VoiceSettings, type VoiceView } from './voice'
import { office } from './office.svelte'
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
  /** "Inserted", shown for a moment after a recording's words are typed. */
  notice = $state('')
  /** A field in the window that takes dictation while it is there: the task in New agent.
   *  With one, a recording is for it; the panel it is in covers every terminal. */
  form = $state.raw<VoiceField | null>(null)
  active = $derived(['preparing', 'listening', 'transcribing'].includes(this.view.phase))
  /** The chosen engine can be used: its model is downloaded, or its provider has a key. */
  ready = $derived(this.settings.engine === 'local'
    ? !this.view.unsupported && this.view.models.some(m => m.id === this.settings.local_model && m.ready)
    : this.view.providers.some(p => p.id === this.settings.engine && p.configured))
  #polling = false
  #alive = false
  #request = 0
  #restoreFocus = false
  /** When the shortcut went down to start the recording, while it is held. */
  #held: number | null = null
  /** Let go after holding while the start was still on its way: stop once it arrives. */
  #stopOnStart = false
  #noticeTimer: ReturnType<typeof setTimeout> | undefined
  /** Counts recordings for a field: a piece of one that was cancelled is not added. */
  #fieldRun = 0
  /** Settles when the field's recording is over and every piece of it has come. */
  #fieldOver: Promise<void> = Promise.resolve()

  start() {
    this.#alive = true
    void bridge.settings().then(s => { if (this.#alive) this.settings = { ...DEFAULT_VOICE, ...s.voice } }).catch(() => {
      if (this.#alive) this.problem = 'Voice settings could not load. Reopen the window to try again.'
    })
    // The core says when anything changes; a slow poll covers an event lost on the way.
    const pushed = bridge.onVoiceView(view => this.#show(view))
    void this.refresh()
    const poll = setInterval(() => {
      if (this.panel || this.active || this.view.busy || this.view.verifying || this.view.downloading) void this.refresh()
    }, bridge.demo ? 200 : 1000)
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
      pushed()
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
      if (request === this.#request) await this.#show(view)
    } catch { /* A destroyed window has nothing to update. Actions surface failures. */ }
    finally { this.#polling = false }
  }
  async #show(view: VoiceView) {
    if (!this.#alive || view.sequence < this.view.sequence) return
    const request = this.#request
    // A recording can end between two updates; the one that says it ended says so once.
    const inserted = ['preparing', 'listening', 'transcribing'].includes(this.view.phase) && view.phase === 'idle' && !!view.message
    this.view = view
    if (inserted) this.#notify(view.message)
    // A recording for a field that is no longer there has nowhere to put its words.
    if (view.field && !this.form && !this.acting && ['preparing', 'listening'].includes(view.phase)) this.cancel()
    // Stop is a button; return its keyboard to the same terminal after insertion.
    // A user who moved elsewhere while waiting keeps their focus.
    if (inserted && view.agent && (this.#restoreFocus || document.activeElement?.closest('.voice-status, .mic'))) {
      const target = view.agent
      await tick()
      if (this.#alive && request === this.#request && (this.#restoreFocus || document.activeElement?.closest('.voice-status, .mic'))) terms.get(target)?.focus()
      this.#restoreFocus = false
    }
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
      await this.act(() => this.view.phase === 'listening' ? bridge.voiceStop() : this.form ? this.#startField(this.form) : this.#startPane(agent))
    } finally { this.acting = false }
  }
  #startPane(agent: string) {
    // Say again which terminals are in front of the user: a recording starts only into one
    // of those, and the core must not be going by an older picture of the room.
    office.reportWatched()
    return bridge.voiceStart(agent)
  }
  async #startField(form: VoiceField) {
    const run = ++this.#fieldRun
    let over = () => {}
    this.#fieldOver = new Promise<void>(resolve => { over = resolve })
    form.begin()
    try {
      await bridge.voiceStartField(message => {
        if (message.done) over()
        else if (run === this.#fieldRun && this.form === form) form.take(message.text, message.space)
      })
    } catch (error) {
      over()
      throw error
    }
    // The field went while the recording was starting.
    if (this.form !== form) this.cancel()
  }
  /** A field offers to take dictation for as long as it is there. Call what is given back
   *  when it goes: a recording for it ends then. */
  offer(form: VoiceField) {
    // What a terminal's recording said is not this field's to show.
    if (this.problem || this.view.phase === 'error') {
      this.problem = ''
      this.cancel()
    }
    this.dismissNotice()
    this.form = form
    return () => {
      if (this.form !== form) return
      this.form = null
      this.problem = ''
      this.dismissNotice()
      if (this.view.field) this.cancel()
    }
  }
  /**
   * Finish the recording for the field and wait until its last words are in it. True when
   * they are, or nothing was being recorded. False when the recording was cancelled
   * meanwhile, the field went, or it ended in trouble: whoever waited must not carry on as
   * if the words were all there, or as if nobody had changed their mind.
   */
  async settle(): Promise<boolean> {
    const wait = (ms: number) => new Promise<false>(resolve => setTimeout(() => resolve(false), ms))
    for (let i = 0; this.acting && i < 100; i++) await wait(30)
    if (!this.view.field || !(this.active || this.view.busy)) return true
    const run = this.#fieldRun
    const form = this.form
    const mine = () => run === this.#fieldRun && this.form === form
    const over = this.#fieldOver.then(() => true as const)
    await this.finish()
    const came = await Promise.race([over, wait(20_000)])
    if (!mine()) return false
    if (!came) {
      this.cancel()
      this.problem = 'The last words did not come in time and were dropped. Check the task, then start again.'
      return false
    }
    // How it ended is said by the view, which can be a moment behind the last word.
    for (let i = 0; i < 40 && mine() && (this.active || this.view.busy); i++) {
      await this.refresh()
      if (this.active || this.view.busy) await wait(50)
    }
    return mine() && this.view.phase !== 'error' && !this.problem
  }
  /** The shortcut went down: start a recording, or finish the one going. Held, it is
   *  push-to-talk (see `release`); tapped, the recording goes on until the next tap. */
  async press(agent: string) {
    if (this.acting || this.changing) return
    if (this.view.phase === 'listening' || this.view.phase === 'preparing') return this.finish()
    if (this.active || this.view.busy) return
    this.#held = Date.now()
    this.#stopOnStart = false
    await this.toggle(agent)
    if (this.#stopOnStart) {
      this.#stopOnStart = false
      await this.finish()
    }
  }
  /** The shortcut came up, or the window lost the keyboard. After a hold, that finishes. */
  release() {
    if (this.#held === null) return
    const held = Date.now() - this.#held
    this.#held = null
    if (held < HOLD_MS) return
    if (this.acting) this.#stopOnStart = true
    else void this.finish()
  }
  /** Finish the recording: the words since the last pause are typed. A recording that has
   *  already ended (its time ran out) needs nothing, and says nothing. */
  async finish() {
    if (this.acting || !['listening', 'preparing'].includes(this.view.phase)) return
    this.#restoreFocus = !!document.activeElement?.closest('.voice-status, .mic')
    this.acting = true
    try { await bridge.voiceStop() } catch { /* It ended on its own; the view shows how. */ }
    finally { this.acting = false }
    await this.refresh()
  }
  #notify(message: string) {
    clearTimeout(this.#noticeTimer)
    this.notice = message
    this.#noticeTimer = setTimeout(() => { this.notice = '' }, 2500)
  }
  dismissNotice() {
    clearTimeout(this.#noticeTimer)
    this.notice = ''
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
    this.#held = null
    this.#stopOnStart = false
    this.#request += 1
    this.#fieldRun += 1
    this.view = { ...this.view, phase: 'idle', agent: null, started_ms: null, message: '', level: 0, transcribing_ms: null, field: false }
    void bridge.voiceCancel().then(() => this.refresh()).catch(() => {
      if (this.#alive) this.problem = 'Voice cancellation could not be confirmed. Close the window to stop capture.'
    })
  }
  download(model: string) { return this.act(() => bridge.voiceDownload(model)) }
  remove(model: string) { return this.act(() => bridge.voiceRemove(model)) }
  cancelDownload() { return this.act(() => bridge.voiceCancelDownload()) }
  /** Keeps a provider's key: checked with the provider first unless `check` is false, in the
   *  keychain unless `sessionOnly`. The key goes to the core and is never sent back. */
  async setKey(provider: VoiceProvider, key: string, sessionOnly: boolean, check: boolean) {
    try {
      await bridge.voiceKeySet(provider, key, sessionOnly, check)
      return null
    } catch (error) { return keyProblem(error) }
    finally { void this.refresh() }
  }
  async checkKey(provider: VoiceProvider) {
    try {
      await bridge.voiceKeyCheck(provider)
      return null
    } catch (error) { return keyProblem(error) }
  }
  removeKey(provider: VoiceProvider) { return this.act(() => bridge.voiceKeyRemove(provider)) }
}
export const voice = new Voice()
