<script lang="ts">
  import { voice } from '../lib/voice.svelte'
  import { voiceKey } from '../lib/voice'
  // One program's terminal. The office draws none of what is in it: the bytes
  // come from the program and go to xterm.js, and the keys go back the other way.
  import { FitAddon } from '@xterm/addon-fit'
  import { WebLinksAddon } from '@xterm/addon-web-links'
  import { Terminal, type ITheme } from '@xterm/xterm'
  import '@xterm/xterm/css/xterm.css'
  import { onMount } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { webAddress } from '../lib/links'
  import { termItems } from '../lib/menus'
  import { office } from '../lib/office.svelte'
  import { findPaths } from '../lib/paths'
  import { backTab, scrolledBack, terms, typedPath } from '../lib/terms'

  interface Props {
    /** The desk whose program this is. */
    id: string
    /** Whether the keyboard goes here. */
    focused: boolean
    /** Behind another pane that has been given the room: kept, not shown. */
    hidden: boolean
    /** Whether its program is running. A terminal that is not there yet is followed once it is. */
    live: boolean
    onfocus: () => void
  }

  let { id, focused, hidden, live, onfocus }: Props = $props()

  let host = $state<HTMLDivElement>()
  let term = $state<Terminal>()
  /** Whether its bytes are being followed. False until there is a terminal to follow. */
  let following = $state(false)
  let connect: () => void = () => {}
  let refit: () => void = () => {}

  /** The terminal's colours, read from the stylesheet so both come from one place. */
  function theme(): ITheme {
    const css = getComputedStyle(host!)
    const colour = (name: string) => css.getPropertyValue(name).trim()
    const ansi = (n: number) => colour(`--ansi-${n}`)
    return {
      background: colour('--term'),
      foreground: colour('--term-ink'),
      cursor: colour('--term-ink'),
      cursorAccent: colour('--term'),
      selectionBackground: colour('--term-select'),
      black: ansi(0),
      red: ansi(1),
      green: ansi(2),
      yellow: ansi(3),
      blue: ansi(4),
      magenta: ansi(5),
      cyan: ansi(6),
      white: ansi(7),
      brightBlack: ansi(8),
      brightRed: ansi(9),
      brightGreen: ansi(10),
      brightYellow: ansi(11),
      brightBlue: ansi(12),
      brightMagenta: ansi(13),
      brightCyan: ansi(14),
      brightWhite: ansi(15)
    }
  }

  /** Keys the office answers to, even in a terminal. Everything else is the program's. */
  function officeKey(event: KeyboardEvent): boolean {
    if (voiceKey(event, voice.settings)) return true
    const { ctrlKey: ctrl, shiftKey: shift, altKey: alt, code } = event
    if (ctrl && code === 'Backquote') return true
    if (ctrl && shift && ['BracketLeft', 'BracketRight', 'KeyW', 'KeyN', 'KeyQ', 'KeyF', 'Enter', 'Slash'].includes(code)) return true
    if (ctrl && !shift && !alt && ['Equal', 'Minus', 'Digit0', 'NumpadAdd', 'NumpadSubtract', 'Numpad0'].includes(code)) return true
    if (alt && !ctrl && !shift && /^Digit[1-9]$/.test(code)) return true
    return false
  }

  onMount(() => {
    if (!host) return
    const css = getComputedStyle(host)
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)')
    const made = new Terminal({
      fontFamily: css.getPropertyValue('--mono').trim() || 'monospace',
      fontSize: office.fontSize,
      lineHeight: 1.2,
      cursorBlink: !reducedMotion.matches,
      scrollback: 5000,
      allowProposedApi: true,
      macOptionIsMeta: true,
      // What is on the screen is also kept as text a screen reader can walk through.
      screenReaderMode: true,
      theme: theme()
    })
    const fit = new FitAddon()
    made.loadAddon(fit)
    // An address is opened in the browser, and only if it is one that may be.
    made.loadAddon(
      new WebLinksAddon((_event, address) => {
        if (webAddress(address)) bridge.openPage(address)
        else office.say('Only https addresses, and http ones on this computer, are opened from a terminal.')
      })
    )
    // A file path is opened in the editor at its line with Ctrl and a click, as in VS Code's terminal.
    made.registerLinkProvider({
      provideLinks(row, callback) {
        const text = made.buffer.active.getLine(row - 1)?.translateToString(true) ?? ''
        const links = findPaths(text).map(found => ({
          range: { start: { x: found.start + 1, y: row }, end: { x: found.start + found.length, y: row } },
          text: found.path,
          activate(event: MouseEvent) {
            if (!(event.ctrlKey || event.metaKey)) {
              const editor = office.editor
              office.hint('file-links', editor ? `Hold Ctrl and click a file path to open it in ${editor.name} at that line.` : 'Hold Ctrl and click a file path to show it in its folder.')
              return
            }
            office.openFile(id, found.path, found.line)
          }
        }))
        callback(links.length > 0 ? links : undefined)
      }
    })
    made.open(host)
    term = made
    const onMotionChange = () => (made.options.cursorBlink = !reducedMotion.matches)
    reducedMotion.addEventListener('change', onMotionChange)
    // Drawing on the graphics card when there is one; the plain way otherwise.
    void import('@xterm/addon-webgl').then(
      ({ WebglAddon }) => {
        if (gone) return
        try {
          const gl = new WebglAddon()
          gl.onContextLoss(() => gl.dispose())
          made.loadAddon(gl)
        } catch {
          // No WebGL here: xterm keeps drawing the way it was.
        }
      },
      () => {}
    )

    let gone = false
    /** Until the screen as it stood has been drawn, nothing is said back to the program. */
    let answering = false
    let unfollow: (() => void) | undefined
    let sized = ''
    let pending: ReturnType<typeof setTimeout> | undefined
    /** Where the last search found something: a line, and the column it starts at. */
    let found: { row: number; col: number } | null = null

    const size = () => {
      if (gone || !host || host.clientWidth < 40 || host.clientHeight < 20) return
      try {
        fit.fit()
      } catch {
        return
      }
      const now = `${made.cols}x${made.rows}`
      if (now !== sized) {
        sized = now
        bridge.resize(id, made.cols, made.rows)
      }
    }
    // A dragged edge resizes many times a second; the program is told once it rests.
    const sizeSoon = () => {
      clearTimeout(pending)
      pending = setTimeout(size, 60)
    }

    made.onData(data => {
      if (answering) bridge.type(id, data)
    })
    made.attachCustomKeyEventHandler(event => {
      if (event.type !== 'keydown') return true
      const mod = event.ctrlKey || event.metaKey
      // The office's own keys: left for the window to act on.
      if (officeKey(event)) return false
      // Shift and Tab is the program's (Claude Code's mode switch). In screen-reader mode
      // xterm sends it without cancelling it, and the browser would move focus out of the
      // terminal; every other key it leaves uncancelled stays in the terminal.
      if (backTab(event)) {
        event.preventDefault()
        made.input('\x1b[Z', true)
        return false
      }
      // Shift and Enter is a new line in the prompt, the way these programs read it.
      if (event.shiftKey && event.key === 'Enter') {
        event.preventDefault()
        bridge.type(id, '\x1b\r')
        return false
      }
      // Copy when something is selected; otherwise Ctrl+C is the program's.
      if (mod && !event.shiftKey && event.code === 'KeyC' && made.hasSelection()) {
        event.preventDefault()
        void navigator.clipboard.writeText(made.getSelection()).catch(() => {})
        made.clearSelection()
        return false
      }
      // Paste is the browser's own event, which xterm takes as a paste.
      if (mod && event.code === 'KeyV') return false
      return true
    })

    // A picture on the clipboard and no text: it is kept in a file and its path is
    // pasted, which Claude Code and Codex both take as the picture.
    const pastePicture = (picture: Blob) =>
      picture
        .arrayBuffer()
        .then(buffer => bridge.savePastedImage(new Uint8Array(buffer)))
        .then(
          path => made.paste(typedPath(path)),
          error => (office.problem = typeof error === 'string' ? error : 'The picture could not be pasted.')
        )
    const onPaste = (event: ClipboardEvent) => {
      const data = event.clipboardData
      if (!data || data.types.includes('text/plain')) return
      const picture = [...data.items].find(item => item.kind === 'file' && item.type.startsWith('image/'))?.getAsFile()
      if (!picture) return
      event.preventDefault()
      event.stopPropagation()
      void pastePicture(picture)
    }
    host.addEventListener('paste', onPaste, true)
    /** What the right-click menu's Paste does: what Ctrl+V does, read from the clipboard itself. */
    const pasteClipboard = async () => {
      const items = await navigator.clipboard.read().catch(() => null)
      if (items === null) {
        const text = await navigator.clipboard.readText()
        if (text) made.paste(text)
        return
      }
      const text = items.find(item => item.types.includes('text/plain'))
      if (text) {
        const words = await (await text.getType('text/plain')).text()
        if (words) made.paste(words)
        return
      }
      const picture = items.find(item => item.types.some(type => type.startsWith('image/')))
      if (picture) await pastePicture(await picture.getType(picture.types.find(type => type.startsWith('image/'))!))
    }

    // Copy on select, when it is turned on: what the mouse selected is copied as the button comes up.
    const onMouseUp = () => {
      if (office.copyOnSelect && made.hasSelection()) void navigator.clipboard.writeText(made.getSelection()).catch(() => {})
    }
    host.addEventListener('mouseup', onMouseUp)

    // Ctrl and the wheel: bigger or smaller text, as in Windows Terminal. A notch at a time,
    // however finely a touchpad turns it.
    let turned = 0
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return
      event.preventDefault()
      event.stopPropagation()
      turned += event.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? event.deltaY : event.deltaY * 100
      while (Math.abs(turned) >= 100) {
        office.setFontSize(turned < 0 ? 1 : -1)
        turned -= Math.sign(turned) * 100
      }
    }
    host.addEventListener('wheel', onWheel, { capture: true, passive: false })

    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault()
      onfocus()
      office.openMenu({ x: event.clientX, y: event.clientY, items: termItems(id) })
    }
    host.addEventListener('contextmenu', onContextMenu)

    terms.set(id, {
      paste: text => made.paste(text),
      pasteClipboard,
      copy: () => navigator.clipboard.writeText(made.getSelection()).then(() => made.clearSelection()),
      hasSelection: () => made.hasSelection(),
      selectAll: () => made.selectAll(),
      clear: () => made.clear(),
      focus: () => made.focus(),
      resetFind: () => {
        found = null
        made.clearSelection()
      },
      find: (text, downward = false) => {
        const needle = text.toLowerCase()
        if (!needle) return false
        const buffer = made.buffer.active
        const last = buffer.length - 1
        const line = (row: number) => buffer.getLine(row)?.translateToString(true).toLowerCase() ?? ''
        // Upward from the last match (or the bottom), or downward from it; round once.
        const startRow = found?.row ?? (downward ? 0 : last)
        for (let step = 0; step <= buffer.length; step++) {
          const row = downward ? (startRow + step) % buffer.length : (startRow - step + buffer.length) % buffer.length
          const words = line(row)
          let col: number
          if (step === 0 && found) col = downward ? words.indexOf(needle, found.col + 1) : found.col > 0 ? words.lastIndexOf(needle, found.col - 1) : -1
          else col = downward ? words.indexOf(needle) : words.lastIndexOf(needle)
          if (col < 0) continue
          found = { row, col }
          made.select(col, row, needle.length)
          made.scrollToLine(Math.max(0, row - Math.floor(made.rows / 2)))
          return true
        }
        return false
      }
    })

    connect = () => {
      if (following || gone) return
      following = true
      void bridge
        .follow(id, (bytes, kept) => {
          if (!kept) {
            made.write(bytes)
            return
          }
          const ready = () => {
            answering = true
            sized = ''
            size()
            // Scrolled back before its pane was put away: back to the same place.
            const line = scrolledBack.get(id)
            scrolledBack.delete(id)
            if (line !== undefined) made.scrollToLine(Math.min(line, made.buffer.active.baseY))
          }
          if (bytes.length === 0) ready()
          else made.write(bytes, ready)
        })
        .then(
          stop => {
            if (gone) stop()
            else unfollow = stop
          },
          // No terminal yet (a desk not started since the office opened): followed once it is.
          () => (following = false)
        )
    }

    refit = sizeSoon
    size()
    connect()

    const watcher = new ResizeObserver(sizeSoon)
    watcher.observe(host)
    const scheme = matchMedia('(prefers-color-scheme: dark)')
    const recolour = () => (made.options.theme = theme())
    scheme.addEventListener('change', recolour)

    return () => {
      gone = true
      clearTimeout(pending)
      // Scrolled back: remembered for when its pane comes back.
      const buffer = made.buffer.active
      if (following && buffer.viewportY < buffer.baseY) scrolledBack.set(id, buffer.viewportY)
      else scrolledBack.delete(id)
      host?.removeEventListener('mouseup', onMouseUp)
      host?.removeEventListener('wheel', onWheel, { capture: true })
      unfollow?.()
      watcher.disconnect()
      scheme.removeEventListener('change', recolour)
      reducedMotion.removeEventListener('change', onMotionChange)
      host?.removeEventListener('paste', onPaste, true)
      host?.removeEventListener('contextmenu', onContextMenu)
      terms.delete(id)
      made.dispose()
    }
  })

  $effect(() => {
    if (focused && !hidden) term?.focus()
  })

  // Its program has started: follow it, if it could not be followed before.
  $effect(() => {
    if (live && !following) connect()
  })

  // Bigger or smaller text: the terminal is fitted to its pane again, and the program told its new size.
  $effect(() => {
    const size = office.fontSize
    if (!term || term.options.fontSize === size) return
    term.options.fontSize = size
    refit()
  })
</script>

<div class="term-host" bind:this={host} onfocusin={onfocus} role="presentation"></div>

<style>
  .term-host {
    min-width: 0;
    min-height: 0;
    height: 100%;
    padding: var(--s-2) 0 0 var(--s-3);
    overflow: hidden;
    background: var(--term);
    /* What is typed and read here is text, and is selected as text. */
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
  .term-host :global(.xterm) {
    height: 100%;
  }
  .term-host :global(.xterm-viewport) {
    scrollbar-color: var(--term-line) transparent;
  }
</style>
