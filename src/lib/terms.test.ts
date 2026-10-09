import { describe, expect, it } from 'vitest'
import { backTab, typedPath } from './terms'

const key = (over: Partial<KeyboardEvent>) => ({ key: 'Tab', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, ...over })

describe('keys a terminal keeps', () => {
  it('takes Shift+Tab, and only Shift+Tab, for the program', () => {
    expect(backTab(key({ shiftKey: true }))).toBe(true)
    // Plain Tab xterm already keeps; with Ctrl or Alt it is the office's or the system's.
    expect(backTab(key({}))).toBe(false)
    expect(backTab(key({ shiftKey: true, ctrlKey: true }))).toBe(false)
    expect(backTab(key({ shiftKey: true, altKey: true }))).toBe(false)
    expect(backTab(key({ key: 'Enter', shiftKey: true }))).toBe(false)
  })
})

describe('a path as typed', () => {
  it('is quoted only when it has a space', () => {
    expect(typedPath('C:\\shop\\a.ts')).toBe('C:\\shop\\a.ts')
    expect(typedPath('C:\\my shop\\a.ts')).toBe('"C:\\my shop\\a.ts"')
  })
})
