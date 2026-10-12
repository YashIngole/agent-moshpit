<script lang="ts">
  // The small menu at the end of the top bar: the layout of the terminals, which
  // programs are on this computer, what version this is, and the way out. Quit
  // lives here as well as in the tray, because not every desktop has a tray.
  import { onMount, tick } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { count } from '../lib/layout'
  import { office } from '../lib/office.svelte'
  import ArrangeItems from './ArrangeItems.svelte'

  let open = $state(false)
  let version = $state('')
  let root = $state<HTMLElement>()
  let width = $state(window.innerWidth)

  const panes = $derived(count(office.layout))
  /** A window too narrow for the bar's Arrange button: what it offers is here instead. */
  const arranges = $derived(panes > 1 && width <= 760)

  onMount(() => {
    void bridge.version().then(
      v => (version = v),
      () => {}
    )
  })

  function close(returnFocus = false) {
    open = false
    if (returnFocus) root?.querySelector<HTMLElement>('button')?.focus()
  }

  function onDocumentPointer(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) close()
  }

  function onKey(event: KeyboardEvent) {
    if (!open) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault()
        void show(event.key === 'ArrowUp')
      }
      return
    }
    if (event.key === 'Escape' && open) {
      event.preventDefault()
      event.stopPropagation()
      close(true)
    } else if (event.key === 'Tab') {
      close(true)
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault()
      event.stopPropagation()
      const list = items()
      const at = list.indexOf(document.activeElement as HTMLElement)
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? list.length - 1 : (at + (event.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length
      list[next]?.focus()
    }
  }

  function items() {
    return [...(root?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])]
  }

  async function show(last = false) {
    open = true
    await tick()
    if (open) { const list = items(); list[last ? list.length - 1 : 0]?.focus() }
  }
</script>

<svelte:window bind:innerWidth={width} />
<svelte:document onpointerdown={onDocumentPointer} />

<div class="menu" bind:this={root} onkeydown={onKey} role="presentation">
  <button type="button" class="more" aria-label="More" aria-haspopup="menu" aria-expanded={open} onclick={() => (open ? close() : void show())}>
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="3" cy="8" r="1.5" /><circle cx="8" cy="8" r="1.5" /><circle cx="13" cy="8" r="1.5" /></svg>
    {#if office.outdated.length > 0 || office.newer}<i class="news" title="Updates are out"></i>{/if}
  </button>
  {#if open}
    <div class="sheet" role="menu">
      {#if office.newer}
        <button
          type="button"
          role="menuitem" tabindex="-1"
          disabled={office.updating}
          onclick={() => {
            close(true)
            void office.updateOffice()
          }}
        >
          Update to {office.newer.version}
          <span>{office.updating ? 'On its way' : `You have ${version}. The office restarts; agents carry on`}</span>
        </button>
        <hr />
      {/if}
      <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(true); office.openPrograms() }}>
        Agent programs
        {#if office.outdated.length > 0}<span>{`${office.outdated.length} update${office.outdated.length === 1 ? '' : 's'} out: ${office.outdated.map(h => h.name).join(', ')}`}</span>{/if}
      </button>
      <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(true); office.openVoice() }}>Voice input</button>
      {#if office.healthProblem}
        <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(true); office.showHealthProblem() }}>Configuration warning</button>
      {/if}
      {#if panes > 0}
        <hr />
        <button type="button" role="menuitemcheckbox" tabindex="-1" aria-checked={office.listed} onclick={() => { close(true); office.toggleCompactFloor() }}>
          <span class="line">Compact floor<i class="tick" aria-hidden="true"></i></span>
        </button>
      {/if}
      {#if arranges}
        <hr />
        <ArrangeItems onchosen={() => close(true)} />
      {/if}
      <hr />
      <button type="button" role="menuitemcheckbox" tabindex="-1" aria-checked={office.copyOnSelect} onclick={() => { close(true); office.setCopyOnSelect(!office.copyOnSelect) }}>
        <span class="line">Copy on select<i class="tick" aria-hidden="true"></i></span>
      </button>
      <button
        type="button"
        role="menuitemcheckbox" tabindex="-1"
        aria-checked={office.closeQuits}
        title={office.closeQuits ? 'Closing the window ends the agents, asking first if anyone is busy' : 'Closing the window leaves the office watching from the tray'}
        onclick={() => { close(true); office.setCloseQuits(!office.closeQuits) }}
      >
        <span class="line">Closing the window quits<i class="tick" aria-hidden="true"></i></span>
      </button>
      <hr />
      <button type="button" role="menuitem" tabindex="-1" aria-keyshortcuts="Control+Shift+/" onclick={() => { close(true); office.openKeys() }}>
        <span class="line">Keys<kbd>Ctrl+Shift+/</kbd></span>
      </button>
      <button type="button" role="menuitem" tabindex="-1" aria-keyshortcuts="Control+Shift+Q" onclick={() => { close(true); bridge.quit() }}>
        <span class="line">Quit Agent Moshpit<kbd>Ctrl+Shift+Q</kbd></span>
      </button>
      <p class="about selectable">agent moshpit {version}</p>
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
    flex: none;
  }
  .more {
    position: relative;
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .more:hover,
  .more[aria-expanded='true'] {
    background: var(--inset);
    color: var(--ink);
  }
  .more svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }

  .sheet {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 10;
    display: grid;
    gap: var(--s-1);
    width: 268px;
    max-width: calc(100vw - 2 * var(--s-3));
    max-height: calc(100dvh - var(--bar) - var(--s-4));
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    animation: unfold var(--quick) var(--ease) both;
    transform-origin: top right;
  }
  @keyframes unfold {
    from {
      transform: scale(0.96) translateY(-4px);
      opacity: 0;
    }
  }
  .sheet > button {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    padding: 7px var(--s-3);
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    font-size: var(--t-md);
    font-weight: 600;
    text-align: left;
  }
  .sheet > button:hover:not(:disabled) {
    background: var(--inset);
  }
  /* The second line under a menu item, only where there is news: an update, what it will do. */
  .sheet > button > span:not(.line) {
    font-size: var(--t-sm);
    font-weight: 400;
    color: var(--ink-2);
  }
  /* A name, and at the end of its line its key or whether it is on. */
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
  }
  .line kbd {
    padding: 0;
    border: 0;
    font-size: var(--t-xs);
    font-weight: 400;
    color: var(--ink-3);
    opacity: 1;
  }
  /* On or off, as a box that is ticked: the same mark as a tick box in a form. */
  .tick {
    flex: none;
    width: 14px;
    height: 14px;
    border: 1px solid var(--field-line);
    border-radius: 4px;
  }
  [aria-checked='true'] .tick {
    border-color: var(--button);
    background:
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'%3E%3Cpath d='M3.5 7.2l2.3 2.3 4.7-5' fill='none' stroke='%230a0c0f' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E") center / 12px no-repeat,
      var(--button);
  }
  hr {
    margin: 2px var(--s-1);
    border: 0;
    border-top: 1px solid var(--line);
  }
  .about {
    margin-top: var(--s-1);
    padding: var(--s-2) var(--s-3) var(--s-1);
    border-top: 1px solid var(--line);
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-3);
  }
  /* A newer version of a program is out: a small mark on the menu, in ink, not a status colour. */
  .news {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink);
  }
</style>
