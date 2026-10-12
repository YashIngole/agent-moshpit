import { describe, expect, it } from 'vitest'
import { cleanVoice, DEFAULT_VOICE, joinPiece, keyProblem, vocabularyWords, voiceKey, voiceKeyUp } from './voice'

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
describe('a dictated piece goes into a text field as typing would put it', () => {
  /** A whole recording into `before|after`: its pieces, each with the space the engine gave it. */
  const dictate = (before: string, after: string, pieces: [string, boolean][]) => {
    let continues = false
    for (const [piece, space] of pieces) {
      const { text, back } = joinPiece(before, after, piece, space, continues)
      before += text.slice(0, text.length - back)
      after = text.slice(text.length - back) + after
      continues = true
    }
    return `${before}|${after}`
  }
  it('starts an empty field with no space, and joins the pieces as the engine says', () => {
    expect(dictate('', '', [['Fix the checkout', false], ['total', true], [', then test it.', false]])).toBe('Fix the checkout total, then test it.|')
    // An engine can settle half a word at a time.
    expect(dictate('', '', [['check', false], ['out', false], ['total', true]])).toBe('checkout total|')
  })
  it('puts one space after what was typed before, and none where one already is', () => {
    expect(dictate('Fix the bug', '', [['and add a test', false]])).toBe('Fix the bug and add a test|')
    expect(dictate('Fix the bug ', '', [['and add a test', false]])).toBe('Fix the bug and add a test|')
    expect(dictate('Fix the bug\n', '', [['Then', false], ['test it', true]])).toBe('Fix the bug\nThen test it|')
    expect(dictate('Fix the bug', '', [[', and test it', false]])).toBe('Fix the bug, and test it|')
    expect(dictate('Look in src/', '', [['lib', false]])).toBe('Look in src/lib|')
    expect(dictate('Rename it (', ')', [['carefully', false]])).toBe('Rename it (carefully|)')
  })
  it('keeps a space before a word that follows the caret, behind the caret', () => {
    expect(dictate('Fix ', 'the bug', [['quick', false], ['ly', false], ['and well', true]])).toBe('Fix quickly and well| the bug')
    expect(dictate('Fix', ' the bug', [['all of', false]])).toBe('Fix all of| the bug')
    expect(dictate('Fix it', '.', [['today', false]])).toBe('Fix it today|.')
  })
  it('treats a piece as a fresh start once the caret has moved', () => {
    expect(joinPiece('Fix the bug', '', 'now', false, false).text).toBe(' now')
    expect(joinPiece('Fix the bug', '', 'now', false, true).text).toBe('now')
    expect(joinPiece('', '', 'Now', true, true).text).toBe('Now')
  })
})
it('word hints are split, cleaned, kept once, and limited', () => {
  expect(vocabularyWords('Moshpit, fix/checkout-total\n  Claude  Code ,, moshpit', ['Moshpit'])).toEqual(['Moshpit', 'fix/checkout-total', 'Claude Code'])
  expect(vocabularyWords('<b>tag</b>\x07')).toEqual(['btag/b'])
  expect(vocabularyWords('x'.repeat(80))[0]).toHaveLength(50)
  expect(vocabularyWords(Array.from({ length: 150 }, (_, i) => `w${i}`).join(','))).toHaveLength(100)
})
