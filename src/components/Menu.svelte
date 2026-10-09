<script lang="ts">
  // The one small menu: a desk's or a terminal's, opened where it was asked for.
  // It can also ask before something final, and hold the box a desk is renamed in.
  import { onMount, tick, untrack } from 'svelte'
  import { office, type Menu, type MenuItem } from '../lib/office.svelte'

  let { menu }: { menu: Menu } = $props()

  let sheet = $state<HTMLElement>()
  // Each menu is drawn afresh (see App.svelte), so where it opens and what it asks are read once.
  let asking = $state<MenuItem | undefined>(untrack(() => menu.asking))
  const renamed = $derived(menu.renaming ? office.agents.find(a => a.id === menu.renaming) : undefined)
  let name = $state('')
  /** Where it is drawn: where it was asked for, kept inside the window. */
  let left = $state(untrack(() => menu.x))
  let top = $state(untrack(() => menu.y))

  onMount(() => {
    name = renamed?.title ?? ''
    void place()
  })

  async function place() {
    await tick()
    if (!sheet) return
    const box = sheet.getBoundingClientRect()
    left = Math.max(8, Math.min(menu.x, window.innerWidth - box.width - 8))
    top = Math.max(8, Math.min(menu.y, window.innerHeight - box.height - 8))
    const first = sheet.querySelector<HTMLElement>('input, [role="menuitem"], .sure .button')
    first?.focus()
    if (first instanceof HTMLInputElement) first.select()
  }

  function choose(item: MenuItem) {
    if (item.confirm && asking !== item) {
      asking = item
      void place()
      return
    }
    office.closeMenu()
    item.run()
  }

  function save() {
    if (menu.renaming && name.trim() !== renamed?.title) office.rename(menu.renaming, name)
    office.closeMenu()
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      office.closeMenu()
      return
    }
    if (event.key === 'Tab') {
      office.closeMenu()
      return
    }
    const keys = ['ArrowDown', 'ArrowUp', 'Home', 'End']
    if (!keys.includes(event.key) || !sheet) return
    const all = [...sheet.querySelectorAll<HTMLElement>('[role="menuitem"]')]
    if (all.length === 0) return
    event.preventDefault()
    const at = all.indexOf(document.activeElement as HTMLElement)
    const next =
      event.key === 'Home' ? 0
      : event.key === 'End' ? all.length - 1
      : event.key === 'ArrowDown' ? (at + 1) % all.length
      : (at - 1 + all.length) % all.length
    all[next]?.focus()
  }

  function onDocumentPointer(event: PointerEvent) {
    if (sheet && !sheet.contains(event.target as Node)) office.closeMenu()
  }
</script>

<svelte:document onpointerdown={onDocumentPointer} />
<svelte:window onblur={() => office.closeMenu()} onresize={() => office.closeMenu()} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="menu-sheet"
  role={renamed ? 'dialog' : 'menu'}
  aria-label={renamed ? `Rename ${renamed.title}` : 'Menu'}
  tabindex="-1"
  style:left="{left}px"
  style:top="{top}px"
  bind:this={sheet}
  onkeydown={onKey}
  oncontextmenu={event => event.preventDefault()}
>
  {#if renamed}
    <form
      class="rename"
      onsubmit={event => {
        event.preventDefault()
        save()
      }}
    >
      <label for="rename-field">Name</label>
      <input id="rename-field" class="field" maxlength="60" bind:value={name} placeholder="Empty: the name it was given" />
      <p>Enter to save, Esc to leave it as it was. Left empty, it goes back to the name it was given.</p>
    </form>
  {:else if asking?.confirm}
    <div class="sure" role="alert">
      <p>{asking.confirm.text}</p>
      <div>
        <button type="button" class="button danger" onclick={() => choose(asking!)}>{asking.confirm.yes}</button>
        <button type="button" class="button quiet" onclick={() => office.closeMenu()}>Keep it</button>
      </div>
    </div>
  {:else}
    {#each menu.items as item (item.label)}
      {#if item.divided}<hr />{/if}
      <button type="button" role="menuitem" class:danger={item.danger} onclick={() => choose(item)}>
        {item.label}
        {#if item.hint}<span>{item.hint}</span>{/if}
      </button>
    {/each}
  {/if}
</div>

<style>
  .menu-sheet {
    position: fixed;
    z-index: 30;
    display: grid;
    gap: 2px;
    width: 280px;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 16px);
    padding: var(--s-2);
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    animation: unfold var(--quick) var(--ease) both;
  }
  @keyframes unfold {
    from {
      transform: scale(0.97) translateY(-3px);
      opacity: 0;
    }
  }
  .menu-sheet:focus {
    outline: none;
  }
  button[role='menuitem'] {
    display: grid;
    padding: 6px var(--s-3);
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    font-size: var(--t-md);
    font-weight: 600;
    text-align: left;
  }
  button[role='menuitem']:hover,
  button[role='menuitem']:focus-visible {
    background: var(--inset);
    outline: none;
  }
  /* What an item does, on a line or two under it: read whole, not cut off. A path breaks anywhere. */
  button[role='menuitem'] span {
    display: -webkit-box;
    overflow: hidden;
    font-size: var(--t-xs);
    font-weight: 400;
    line-height: 1.35;
    color: var(--ink-3);
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  button.danger {
    color: var(--trouble-ink);
  }
  hr {
    margin: var(--s-1) 0;
    border: 0;
    border-top: 1px solid var(--line);
  }
  .sure {
    display: grid;
    gap: var(--s-2);
    padding: var(--s-2);
    border-radius: var(--r-1);
    background: var(--trouble-wash);
    font-size: var(--t-sm);
    line-height: 1.45;
  }
  .sure div {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
  }
  .rename {
    display: grid;
    gap: 6px;
    padding: var(--s-1);
  }
  .rename label {
    font-size: var(--t-xs);
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .rename p {
    font-size: var(--t-xs);
    color: var(--ink-3);
  }
</style>
