import { expect, test } from 'vitest'
import { menuClick, withClickKey } from './platform'

const click = (ctrlKey: boolean, metaKey: boolean) => ({ ctrlKey, metaKey })

test('on a Mac, Cmd asks for more and a Control-click is only the menu', () => {
  expect(withClickKey(click(false, true), true)).toBe(true)
  expect(withClickKey(click(true, false), true)).toBe(false)
  expect(menuClick(click(true, false), true)).toBe(true)
  // Both held is still the system's right-click.
  expect(withClickKey(click(true, true), true)).toBe(false)
  expect(menuClick(click(false, true), true)).toBe(false)
  expect(menuClick(click(false, false), true)).toBe(false)
})

test('elsewhere, Ctrl asks for more and no click is the menu', () => {
  expect(withClickKey(click(true, false), false)).toBe(true)
  expect(withClickKey(click(false, true), false)).toBe(true)
  expect(withClickKey(click(false, false), false)).toBe(false)
  expect(menuClick(click(true, false), false)).toBe(false)
})
