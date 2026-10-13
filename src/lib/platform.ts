// Where the systems the office runs on differ, and the window has to know.

/** macOS. A Control-click there is the system's right-click, so the click that asks for more is Cmd's. */
export const MAC = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform)

/** The key held with a click to ask for more: a desk beside the others, a file path in the editor. */
export const CLICK_KEY = MAC ? 'Cmd' : 'Ctrl'

type Held = Pick<MouseEvent, 'ctrlKey' | 'metaKey'>

/** A click made with that key held. On a Mac, Control is not it: that click is the menu's. */
export function withClickKey(event: Held, mac = MAC): boolean {
  return mac ? event.metaKey && !event.ctrlKey : event.ctrlKey || event.metaKey
}

/**
 * A Mac's Control-click. The system sends it as a right-click, which opens the menu, and
 * sends the click as well: whatever takes clicks must leave this one alone, or it is done twice.
 */
export function menuClick(event: Held, mac = MAC): boolean {
  return mac && event.ctrlKey
}
