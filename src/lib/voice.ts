/** Where the words come from: this computer, or a cloud provider with the user's own key. */
export type VoiceEngine = 'local' | 'deepgram' | 'openai' | 'assemblyai' | 'elevenlabs' | 'openrouter'
export type VoiceProvider = Exclude<VoiceEngine, 'local'>
export interface VoiceSettings {
  enabled: boolean
  engine: VoiceEngine
  /** The local model's id, from the view's `models`. */
  local_model: string
  shortcut: 'space' | 'altspace' | 'none'
  microphone: string | null
  /** The user's own words for a cloud provider to expect: names, jargon. */
  vocabulary: string[]
  /** A cloud provider is also told the desk's name, folder and branch. */
  send_desk_names: boolean
}
export const DEFAULT_VOICE: VoiceSettings = { enabled: false, engine: 'local', local_model: 'nemotron-streaming-en-q8', shortcut: 'space', microphone: null, vocabulary: [], send_desk_names: true }
export interface VoiceInputs { devices: string[]; default: string | null }
export interface VoiceModelView { id: string; label: string; bytes: number; ready: boolean; recommended: boolean }
export interface VoiceProviderView {
  id: VoiceProvider
  name: string
  /** A key is kept for it, in the system keychain or for this session only. The key itself never comes this way. */
  configured: boolean
  last4: string
  session_only: boolean
}
export interface VoiceView {
  sequence: number
  phase: 'idle' | 'preparing' | 'listening' | 'transcribing' | 'error'
  agent: string | null
  started_ms: number | null
  message: string
  /** The engine of the recording shown. */
  engine: VoiceEngine
  /** What the engine is doing before it hears: loading, connecting. Empty once it hears. */
  detail: string
  models: VoiceModelView[]
  downloading: string | null
  received: number
  download_error: string
  verifying: boolean
  busy: boolean
  level: number
  microphone: string
  transcribing_ms: number | null
  /** The words still being decided, after those already typed. */
  partial: string
  /** The recording shown is for a field in the window (the task in New agent), not a terminal. */
  field: boolean
  /** Why this computer cannot run the local engine; empty when it can. Cloud engines need nothing. */
  unsupported: string
  providers: VoiceProviderView[]
}
export const EMPTY_VOICE: VoiceView = { sequence: 0, phase: 'idle', agent: null, started_ms: null, message: '', engine: 'local', detail: '', models: [], downloading: null, received: 0, download_error: '', verifying: false, busy: false, level: 0, microphone: '', transcribing_ms: null, partial: '', field: false, unsupported: '', providers: [] }
/** A settled piece of a recording for a field, and whether a space stands between it and the
 *  piece before. `done` comes last, once, however the recording ended: nothing follows it. */
export interface FieldText { text: string; space: boolean; done: boolean }
/** A field in the window that takes dictation while it is there. */
export interface VoiceField {
  /** A recording is starting: show where its words will go. */
  begin(): void
  take(text: string, space: boolean): void
}

/**
 * What the panel says about each provider. Prices change, so they are linked, not quoted.
 * `phrased`: it is sent each phrase whole, at the pause after it, and has no words to show
 * while one is being said. `hints: false`: it is not told the words to expect.
 */
export const PROVIDERS: Record<VoiceProvider, { model: string; keys: string; pricing: string; privacy: string; phrased?: true; hints?: false }> = {
  deepgram: { model: 'Nova-3', keys: 'https://console.deepgram.com/', pricing: 'https://deepgram.com/pricing', privacy: 'Agent Moshpit asks Deepgram not to use your audio to improve its models.' },
  openai: { model: 'gpt-live-transcribe', keys: 'https://platform.openai.com/api-keys', pricing: 'https://openai.com/api/pricing/', privacy: 'OpenAI does not train on API audio unless your organization opts in.' },
  assemblyai: { model: 'Universal-Streaming English', keys: 'https://www.assemblyai.com/dashboard', pricing: 'https://www.assemblyai.com/pricing', privacy: 'Check your AssemblyAI account settings for how it keeps and uses audio.' },
  elevenlabs: { model: 'Scribe v2 Realtime', keys: 'https://elevenlabs.io/app/settings/api-keys', pricing: 'https://elevenlabs.io/pricing/api', privacy: 'ElevenLabs keeps request logs unless your plan has zero retention. Word hints cost extra with ElevenLabs.' },
  openrouter: { model: 'ElevenLabs Scribe v2', keys: 'https://openrouter.ai/settings/keys', pricing: 'https://openrouter.ai/elevenlabs/scribe-v2', privacy: 'What OpenRouter and ElevenLabs keep of it follows your OpenRouter privacy settings.', phrased: true, hints: false }
}
export const isProvider = (engine: VoiceEngine): engine is VoiceProvider => engine !== 'local'
/** The engine types a phrase at a time, a moment after each pause, and shows no words meanwhile. */
export const isPhrased = (engine: VoiceEngine): boolean => engine !== 'local' && !!PROVIDERS[engine].phrased
export function engineName(engine: VoiceEngine): string {
  return engine === 'local' ? 'this computer' : ({ deepgram: 'Deepgram', openai: 'OpenAI', assemblyai: 'AssemblyAI', elevenlabs: 'ElevenLabs', openrouter: 'OpenRouter' } as const)[engine]
}

/** A key error, without the marker that says what the window can offer instead: keep the key
 *  for this session when the keychain is missing, or keep it unchecked when the provider could
 *  not be reached. */
export function keyProblem(error: unknown): { text: string; offer: 'session' | 'unchecked' | null } {
  const text = typeof error === 'string' ? error : error instanceof Error ? error.message : 'The key could not be kept. Try again.'
  if (text.startsWith('keychain:')) return { text: text.slice(9), offer: 'session' }
  if (text.startsWith('unreached:')) return { text: text.slice(10), offer: 'unchecked' }
  return { text, offer: null }
}

/** Words typed into the vocabulary box, as the list kept: split at commas and new lines, cleaned,
 *  each once, at most 100 of 50 characters. */
export function vocabularyWords(text: string, kept: string[] = []): string[] {
  const seen = new Set<string>()
  return [...kept, ...text.split(/[,\n]/)]
    .map(word => word.replace(/[\x00-\x1f\x7f<>]/g, '').replace(/\s+/g, ' ').trim().slice(0, 50).trim())
    .filter(word => word && !seen.has(word.toLowerCase()) && !!seen.add(word.toLowerCase()))
    .slice(0, 100)
}

export function voiceKey(event: Pick<KeyboardEvent, 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey' | 'code'>, settings: VoiceSettings): boolean {
  return settings.enabled && settings.shortcut !== 'none' && event.ctrlKey && event.shiftKey && !event.metaKey && event.code === 'Space' && event.altKey === (settings.shortcut === 'altspace')
}
/** Held at least this long, the shortcut is push-to-talk and letting go finishes. A shorter tap
 *  leaves the microphone listening hands-free until the next tap. */
export const HOLD_MS = 350
/** Letting go of any key of the shortcut ends a push-to-talk recording. */
export function voiceKeyUp(event: Pick<KeyboardEvent, 'code'>, settings: VoiceSettings): boolean {
  if (!settings.enabled || settings.shortcut === 'none') return false
  return event.code === 'Space' || /^(Control|Shift)(Left|Right)$/.test(event.code) || (settings.shortcut === 'altspace' && /^Alt(Left|Right)$/.test(event.code))
}
export function shortcutLabel(shortcut: VoiceSettings['shortcut']): string {
  return shortcut === 'none' ? 'No shortcut' : shortcut === 'altspace' ? 'Ctrl+Alt+Shift+Space' : 'Ctrl+Shift+Space'
}
/**
 * A dictated piece as it goes into a text field at the caret, between `before` and `after`.
 * A piece that carries on from the last one (`continues`) has the space the engine says it
 * has: an engine can settle half a word at a time. One that starts somewhere fresh takes a
 * space unless it starts the text, follows a space, an opening bracket or a slash, or opens
 * with punctuation. When a word follows the caret, a space is put between, behind the caret
 * (`back`), so the next piece still joins this one. Never two spaces.
 */
export function joinPiece(before: string, after: string, piece: string, space: boolean, continues: boolean): { text: string; back: number } {
  const lead = !before || /\s$/.test(before) ? false : continues ? space : !/[([{“‘\/]$/.test(before) && !/^[,.;:!?)\]}%]/.test(piece)
  const trail = /^[^\s,.;:!?)\]}]/.test(after)
  return { text: (lead ? ' ' : '') + piece + (trail ? ' ' : ''), back: trail ? 1 : 0 }
}
/** Demo mirrors the terminal's plain-text safety boundary. The native boundary is authoritative. */
export function cleanVoice(text: string): string {
  return text.replace(/\x1b(?:\[[0-?]*[ -/]*[@-~]|[\]PX^_][\s\S]*?(?:\x07|\x1b\\)|[ -/]*[@-~])/g, '').replace(/[\x00-\x08\x0e-\x1f\x7f-\x9f\u202a-\u202e\u2066-\u2069]/g, '').replace(/\s+/g, ' ').trim()
}
