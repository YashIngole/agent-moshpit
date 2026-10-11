import { describe, expect, it } from 'vitest'
import { cleanVoice, DEFAULT_VOICE, keyProblem, vocabularyWords, voiceKey, voiceKeyUp } from './voice'

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
it('letting go of any key of the shortcut finishes a held recording, and nothing else does', () => {
  const settings = { ...DEFAULT_VOICE, enabled: true }
  for (const code of ['Space', 'ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight']) expect(voiceKeyUp({ code }, settings)).toBe(true)
  for (const code of ['KeyA', 'Enter', 'AltLeft', 'MetaLeft']) expect(voiceKeyUp({ code }, settings)).toBe(false)
  expect(voiceKeyUp({ code: 'AltLeft' }, { ...settings, shortcut: 'altspace' })).toBe(true)
  expect(voiceKeyUp({ code: 'Space' }, { ...settings, shortcut: 'none' })).toBe(false)
  expect(voiceKeyUp({ code: 'Space' }, DEFAULT_VOICE)).toBe(false)
})
it('demo text reaches a prompt without controls, line breaks, or clipboard escape sequences', () => {
  expect(cleanVoice('Fix\r\nthe\tfile\x1b[31m\x1b[0m\x03')).toBe('Fix the file')
  expect(cleanVoice('a\x1b]52;c;secret\x07b नमस्ते')).toBe('ab नमस्ते')
})
it('a key error says what the window can offer instead, without its marker', () => {
  expect(keyProblem('keychain:The system keychain could not be opened.')).toEqual({ text: 'The system keychain could not be opened.', offer: 'session' })
  expect(keyProblem('unreached:Deepgram could not be reached to check the key.')).toEqual({ text: 'Deepgram could not be reached to check the key.', offer: 'unchecked' })
  expect(keyProblem('Deepgram did not accept the key.')).toEqual({ text: 'Deepgram did not accept the key.', offer: null })
  expect(keyProblem(new Error('boom')).text).toBe('boom')
})
it('word hints are split, cleaned, kept once, and limited', () => {
  expect(vocabularyWords('Moshpit, fix/checkout-total\n  Claude  Code ,, moshpit', ['Moshpit'])).toEqual(['Moshpit', 'fix/checkout-total', 'Claude Code'])
  expect(vocabularyWords('<b>tag</b>\x07')).toEqual(['btag/b'])
  expect(vocabularyWords('x'.repeat(80))[0]).toHaveLength(50)
  expect(vocabularyWords(Array.from({ length: 150 }, (_, i) => `w${i}`).join(','))).toHaveLength(100)
})
