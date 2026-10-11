export type VoiceModel = 'small' | 'base'
export interface VoiceSettings {
  enabled: boolean
  model: VoiceModel
  language: 'auto' | 'english' | 'hindi'
  shortcut: 'space' | 'altspace' | 'none'
  microphone: string | null
}
export const DEFAULT_VOICE: VoiceSettings = { enabled: false, model: 'base', language: 'auto', shortcut: 'space', microphone: null }
export interface VoiceInputs { devices: string[]; default: string | null }
export interface VoiceView {
  sequence: number
  phase: 'idle' | 'preparing' | 'listening' | 'transcribing' | 'error'
  agent: string | null
  started_ms: number | null
  message: string
  models: { id: VoiceModel; bytes: number; ready: boolean }[]
  downloading: VoiceModel | null
  received: number
  download_error: string
  verifying: boolean
  busy: boolean
  level: number
  microphone: string
  transcribing_ms: number | null
  /** The words of the phrase still being spoken, before they are typed. */
  partial: string
  /** Why this computer cannot run local voice; empty when it can. */
  unsupported: string
}
export const EMPTY_VOICE: VoiceView = { sequence: 0, phase: 'idle', agent: null, started_ms: null, message: '', models: [], downloading: null, received: 0, download_error: '', verifying: false, busy: false, level: 0, microphone: '', transcribing_ms: null, partial: '', unsupported: '' }
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
/** Demo mirrors the terminal's plain-text safety boundary. The native boundary is authoritative. */
export function cleanVoice(text: string): string {
  return text.replace(/\x1b(?:\[[0-?]*[ -/]*[@-~]|[\]PX^_][\s\S]*?(?:\x07|\x1b\\)|[ -/]*[@-~])/g, '').replace(/[\x00-\x08\x0e-\x1f\x7f-\x9f\u202a-\u202e\u2066-\u2069]/g, '').replace(/\s+/g, ' ').trim()
}
