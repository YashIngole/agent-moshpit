import { describe, expect, it } from 'vitest'
import { cleanVoice, DEFAULT_VOICE, voiceKey } from './voice'

describe('voice keys respect terminal ownership', () => {
  const key = { ctrlKey: true, shiftKey: true, altKey: false, metaKey: false, code: 'Space' }
  it('reserves no key until enabled, and supports disabling or changing the shortcut', () => {
    expect(voiceKey(key, DEFAULT_VOICE)).toBe(false)
    expect(voiceKey(key, { ...DEFAULT_VOICE, enabled: true })).toBe(true)
    expect(voiceKey(key, { ...DEFAULT_VOICE, enabled: true, shortcut: 'none' })).toBe(false)
    expect(voiceKey(key, { ...DEFAULT_VOICE, enabled: true, shortcut: 'altspace' })).toBe(false)
    expect(voiceKey({ ...key, altKey: true }, { ...DEFAULT_VOICE, enabled: true, shortcut: 'altspace' })).toBe(true)
  })
  it('does not consume plain Space or existing office keys', () => {
    const settings = { ...DEFAULT_VOICE, enabled: true }
    for (const code of ['Backquote', 'KeyN', 'KeyW', 'KeyQ', 'KeyF', 'Enter', 'Slash', 'BracketLeft', 'BracketRight', 'KeyV']) expect(voiceKey({ ...key, code }, settings)).toBe(false)
    expect(voiceKey({ ...key, ctrlKey: false }, settings)).toBe(false)
    expect(voiceKey({ ...key, metaKey: true }, settings)).toBe(false)
  })
})
it('demo text reaches a prompt without controls, line breaks, or clipboard escape sequences', () => {
  expect(cleanVoice('Fix\r\nthe\tfile\x1b[31m\x1b[0m\x03')).toBe('Fix the file')
  expect(cleanVoice('a\x1b]52;c;secret\x07b नमस्ते')).toBe('ab नमस्ते')
})
