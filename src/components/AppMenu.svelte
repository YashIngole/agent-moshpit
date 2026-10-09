<script lang="ts">
  // The small menu at the end of the top bar: the layout of the terminals, which
  // programs are on this computer, what version this is, and the way out. Quit
  // lives here as well as in the tray, because not every desktop has a tray.
  import { onMount, tick } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { ids } from '../lib/layout'
  import { office } from '../lib/office.svelte'

  let open = $state(false)
  let version = $state('')
  let root = $state<HTMLElement>()

  const panes = $derived(ids(office.layout).length)

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
          <span>{office.updating ? 'On its way' : `You have ${version}. The office starts again; agents carry on where their programs can`}</span>
        </button>
      {/if}
      <button
        type="button"
        role="menuitem" tabindex="-1"
        onclick={() => {
          close(true)
          office.openPrograms()
        }}
      >
        Agent programs
        <span>{office.outdated.length > 0 ? `${office.outdated.length} update${office.outdated.length === 1 ? '' : 's'} out: ${office.outdated.map(h => h.name).join(', ')}` : 'Install, update, see versions'}</span>
      </button>
      <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(true); office.openVoice() }}>
        Voice input
        <span>Local dictation, model downloads and language</span>
      </button>
      {#if office.healthProblem}
        <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(true); office.showHealthProblem() }}>Configuration warning<span>Open the file or dismiss the warning</span></button>
      {/if}
      {#if panes > 0}
        <button type="button" role="menuitemcheckbox" tabindex="-1" aria-checked={office.listed} onclick={() => { close(true); office.toggleCompactFloor() }}>Compact floor<span>{office.listed ? 'Use the larger desks' : 'Give terminals more room'}</span></button>
      {/if}
      {#if panes > 1}
        <button
          type="button"
          role="menuitem" tabindex="-1"
          onclick={() => {
            close(true)
            office.evenOut()
          }}
        >
          Make the terminals even
          <span>{panes} open, each an equal share</span>
        </button>
      {/if}
      <button
        type="button"
        role="menuitemcheckbox" tabindex="-1"
        aria-checked={office.copyOnSelect}
        onclick={() => {
          close(true)
          office.setCopyOnSelect(!office.copyOnSelect)
        }}
      >
        Copy on select
        <span>{office.copyOnSelect ? 'On · selecting text in a terminal copies it' : 'Off · Ctrl+C copies what is selected'}</span>
      </button>
      <button
        type="button"
        role="menuitemcheckbox" tabindex="-1"
        aria-checked={office.closeQuits}
        onclick={() => {
          close(true)
          office.setCloseQuits(!office.closeQuits)
        }}
      >
        Closing the window quits
        <span>{office.closeQuits ? 'On · asking first if anyone is busy' : 'Off · the office keeps watching from the tray'}</span>
      </button>
      <button
        type="button"
        role="menuitem" tabindex="-1"
        onclick={() => {
          close(true)
          office.openKeys()
        }}
      >
        Keys
        <span>Every shortcut, in one list. Ctrl+Shift+/</span>
      </button>
      <button
        type="button"
        role="menuitem" tabindex="-1"
        onclick={() => {
          close(true)
          bridge.quit()
        }}
      >
        Quit Agent Moshpit
        <span>Ends the agents' programs, asking first if anyone is busy. Ctrl+Shift+Q</span>
      </button>
      <p class="about selectable">agent moshpit {version} · {office.closeQuits ? 'closing the window quits' : 'closing the window leaves it running in the tray'}</p>
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
    padding: var(--s-2) var(--s-3);
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    font-size: var(--t-md);
    font-weight: 650;
    text-align: left;
  }
  .sheet > button:hover:not(:disabled) {
    background: var(--inset);
  }
  /* The second line under a menu item: what it does. */
  .sheet > button span {
    font-size: var(--t-sm);
    font-weight: 400;
    color: var(--ink-2);
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
