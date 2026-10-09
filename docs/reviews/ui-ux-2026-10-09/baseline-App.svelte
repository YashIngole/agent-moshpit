<script lang="ts">
  import { onMount } from 'svelte'
  import Floor from './components/Floor.svelte'
  import Keys from './components/Keys.svelte'
  import KnockBand from './components/KnockBand.svelte'
  import Menu from './components/Menu.svelte'
  import NewAgent from './components/NewAgent.svelte'
  import Panes from './components/Panes.svelte'
  import Programs from './components/Programs.svelte'
  import Toast from './components/Toast.svelte'
  import TopBar from './components/TopBar.svelte'
  import { startBeat } from './lib/beat'
  import { bridge, type Drop } from './lib/bridge'
  import { FLOOR_WIDTH, office } from './lib/office.svelte'
  import { terms, typedPath } from './lib/terms'

  onMount(() => {
    const stopOffice = office.start()
    const stopBeat = startBeat()
    const stopDrop = bridge.onDrop(onDrop)
    return () => {
      stopOffice()
      stopBeat()
      stopDrop()
    }
  })

  /** The pane at a place in the window, if there is one there. */
  function paneAt(x: number, y: number): string {
    const pane = document.elementsFromPoint(x, y).find(el => el instanceof HTMLElement && el.matches('.pane'))
    return pane instanceof HTMLElement ? (pane.dataset.pane ?? '') : ''
  }

  /** Files dropped on a pane: their paths are pasted into its program, as a terminal does. */
  function onDrop(drop: Drop) {
    if (drop.kind === 'leave') {
      office.dropTarget = ''
      return
    }
    const id = paneAt(drop.x, drop.y)
    if (drop.kind === 'over') {
      office.dropTarget = terms.has(id) ? id : ''
      return
    }
    office.dropTarget = ''
    const term = terms.get(id)
    if (!term) {
      if (drop.paths.length > 0) office.say('Drop files on a terminal to paste their paths into it.')
      return
    }
    office.focusPane(id)
    term.paste(drop.paths.map(typedPath).join(' '))
    term.focus()
  }

  /** The browser's own right-click menu (Back, Refresh, Print) has no place here; text fields keep theirs. */
  function onContextMenu(event: MouseEvent) {
    if (!(event.target instanceof HTMLElement && event.target.closest('input, textarea, [contenteditable]'))) event.preventDefault()
  }

  function typing(target: EventTarget | null) {
    return target instanceof HTMLElement && target.matches('input, textarea, select, [contenteditable]')
  }

  function inTerminal(target: EventTarget | null) {
    return target instanceof HTMLElement && target.closest('.term-host') !== null
  }

  /** The office's own keys, which work even in a terminal (see TerminalView's list). True when one was used. */
  function officeKey(event: KeyboardEvent): boolean {
    const { ctrlKey: ctrl, shiftKey: shift, altKey: alt, code } = event
    const pane = office.focused && office.open.has(office.focused) ? office.focused : ''
    // Back to the floor and back again. Not Escape: a program in a terminal uses that itself.
    if (ctrl && code === 'Backquote') office.toggleTerminals()
    else if (ctrl && shift && (code === 'BracketLeft' || code === 'BracketRight')) office.stepPane(code === 'BracketRight' ? 1 : -1)
    else if (ctrl && shift && code === 'KeyW') pane && office.closePane(pane)
    else if (ctrl && shift && code === 'Enter') pane && office.toggleZoom(pane)
    else if (ctrl && shift && code === 'KeyF') pane && (office.finding = pane)
    else if (ctrl && shift && code === 'KeyN') office.openNew()
    else if (ctrl && shift && code === 'KeyQ') bridge.quit()
    else if (ctrl && shift && code === 'Slash') office.openKeys()
    else if (ctrl && !shift && !alt && (code === 'Equal' || code === 'NumpadAdd')) office.setFontSize(1)
    else if (ctrl && !shift && !alt && (code === 'Minus' || code === 'NumpadSubtract')) office.setFontSize(-1)
    else if (ctrl && !shift && !alt && (code === 'Digit0' || code === 'Numpad0')) office.setFontSize(0)
    else if (alt && !ctrl && !shift && /^Digit[1-9]$/.test(code) && office.terminals) office.focusNth(Number(code.slice(5)))
    else return false
    return true
  }

  function onKey(event: KeyboardEvent) {
    if (officeKey(event)) {
      event.preventDefault()
      return
    }
    // Everything else typed in a terminal belongs to its program.
    if (inTerminal(event.target)) return
    if (event.key === 'Escape' && office.panel) {
      event.preventDefault()
      office.closePanel()
      return
    }
    if (event.ctrlKey && !event.shiftKey && event.code === 'KeyQ') {
      event.preventDefault()
      bridge.quit()
      return
    }
    if (event.ctrlKey || event.metaKey || event.altKey || typing(event.target)) return
    if (event.key.toLowerCase() === 'n') {
      event.preventDefault()
      office.openNew()
    } else if (event.key === '?') {
      event.preventDefault()
      office.openKeys()
    }
  }

  // The edge between the floor and the terminals.
  let dragging = $state(false)

  function onGripDown(event: PointerEvent) {
    if (event.button !== 0) return
    event.preventDefault()
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    dragging = true
  }

  function onGripMove(event: PointerEvent) {
    if (dragging) office.setFloorWidth(event.clientX, false)
  }

  function onGripUp() {
    if (!dragging) return
    dragging = false
    office.setFloorWidth(office.floorWidth)
  }

  function onGripKey(event: KeyboardEvent) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    office.setFloorWidth(office.floorWidth + (event.key === 'ArrowRight' ? 24 : -24))
  }
</script>

<svelte:window onkeydown={onKey} oncontextmenu={onContextMenu} />

<div class="app" class:terminals={office.terminals} class:paneled={office.panel !== null} class:dragging>
  <TopBar />
  {#if office.problem}
    <p class="problem-strip" role="alert">
      <span>{office.problem}</span>
      <span class="actions">
        {#if office.problemFile}
          {@const file = office.problemFile}
          <button type="button" class="button quiet" onclick={() => office.openFile(null, file.path, file.line)}>
            Open {file.path.split(/[\\/]/).pop()}
          </button>
        {/if}
        <button type="button" class="button quiet" onclick={() => (office.problem = '')}>Dismiss</button>
      </span>
    </p>
  {/if}
  <KnockBand />
  <div class="work" style:--floor="{office.floorWidth}px">
    <Floor />
    {#if office.terminals}
      <!-- A separator that can be moved is a control: it takes focus and the arrow keys. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="grip"
        role="separator"
        tabindex="0"
        aria-orientation="vertical"
        aria-label="Width of the floor"
        aria-valuemin={FLOOR_WIDTH.least}
        aria-valuenow={office.floorWidth}
        aria-valuetext="{office.floorWidth} pixels"
        title="Drag to resize. Double-click for the strip of desks."
        onpointerdown={onGripDown}
        onpointermove={onGripMove}
        onpointerup={onGripUp}
        onpointercancel={onGripUp}
        ondblclick={() => office.setFloorWidth(office.floorWidth <= FLOOR_WIDTH.strip ? FLOOR_WIDTH.usual : FLOOR_WIDTH.strip)}
        onkeydown={onGripKey}
      ></div>
      <Panes />
    {/if}
    {#if office.panel?.kind === 'new'}
      <NewAgent cwd={office.panel.cwd} harness={office.panel.harness} />
    {:else if office.panel?.kind === 'programs'}
      <Programs />
    {:else if office.panel?.kind === 'keys'}
      <Keys />
    {/if}
    <!-- Said over the floor, never over the foot of a terminal, where a program's prompt and choices are. -->
    {#if office.toast}
      <Toast toast={office.toast} beside={office.terminals} />
    {/if}
  </div>
</div>
{#if office.menu}
  {#key office.menu}
    <Menu menu={office.menu} />
  {/key}
{/if}

<style>
  /* A column: bar, notices when there are any, then the floor takes whatever is left. */
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .app > :global(*) {
    flex: none;
    min-width: 0;
  }
  .work {
    position: relative;
    display: grid;
    flex: 1 1 0;
    grid-template-columns: minmax(0, 1fr) auto;
    min-height: 0;
  }

  /* Terminals open: the floor keeps the width it was dragged to, and they take the rest. */
  .terminals .work {
    grid-template-columns: min(var(--floor), 100% - 420px) minmax(0, 1fr);
  }
  .terminals .work :global(.floor) {
    gap: var(--s-3);
    padding: var(--s-3);
  }
  /* The New agent form lies over the terminals rather than squeezing them. */
  .terminals .work :global(.panel) {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 5;
    box-shadow: var(--lift);
  }

  /* The edge between the floor and the terminals. Wider to the hand than to the eye. */
  .grip {
    position: absolute;
    top: 0;
    bottom: 0;
    left: min(var(--floor), 100% - 420px);
    z-index: 4;
    width: 9px;
    margin-left: -5px;
    cursor: col-resize;
    touch-action: none;
  }
  .grip::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 3px;
    width: 3px;
    background: transparent;
    transition: background var(--quick) var(--ease);
  }
  .grip:hover::after,
  .grip:focus-visible::after,
  .dragging .grip::after {
    background: var(--wall);
  }
  .grip:focus-visible {
    outline: none;
  }
  .grip:focus-visible::after {
    background: var(--focus);
  }
  /* While the edge is held, the terminals under the pointer must not take it. */
  .dragging .work :global(.panes) {
    pointer-events: none;
  }

  .problem-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-3) var(--s-2) var(--s-5);
    background: var(--trouble-wash);
    color: var(--trouble-ink);
    font-size: var(--t-sm);
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .problem-strip .actions {
    display: flex;
    flex: none;
    gap: var(--s-2);
  }

  /* In a narrow window the terminals, or the form, take the room and the floor waits behind. */
  @media (max-width: 760px) {
    .terminals .work,
    .paneled .work {
      grid-template-columns: minmax(0, 1fr);
    }
    .terminals .work :global(.floor),
    .paneled .work :global(.floor),
    .grip {
      display: none;
    }
    .work :global(.panel) {
      width: 100%;
      border-left: 0;
    }
  }
</style>
