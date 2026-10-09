// The terminals on screen, by desk, for what reaches them from outside their pane:
// a right-click menu, a dropped file, the find bar.

export interface TermHandle {
  /** Hand text to the program as a paste, the way Ctrl+V does. */
  paste(text: string): void
  /** Paste what is on the clipboard, as Ctrl+V does: text as text, a picture as the path of a file holding it. */
  pasteClipboard(): Promise<void>
  copy(): Promise<void>
  hasSelection(): boolean
  selectAll(): void
  /** Wipe what is kept above the screen. The program's own screen stays. */
  clear(): void
  focus(): void
  /**
   * Find text in what the terminal has shown, upward from the last match (or from
   * the bottom), or downward. Selects and shows it. False when there is none.
   */
  find(text: string, downward?: boolean): boolean
  /** Forget where the last search stopped. */
  resetFind(): void
}

export const terms = new Map<string, TermHandle>()

/**
 * Where a terminal was scrolled back to when its pane was put away, by desk: the line at
 * the top of its view. A pane that comes back (from the floor, say) opens there again.
 */
export const scrolledBack = new Map<string, number>()

/**
 * Shift+Tab alone: the one key xterm, in screen-reader mode, sends to the program
 * without cancelling, whose default the browser acts on by moving focus.
 */
export function backTab(event: Pick<KeyboardEvent, 'key' | 'shiftKey' | 'ctrlKey' | 'altKey' | 'metaKey'>): boolean {
  return event.key === 'Tab' && event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey
}

/** A path as a person would type it in a terminal: in quotes when it has a space. */
export function typedPath(path: string): string {
  return /\s/.test(path) ? `"${path}"` : path
}
