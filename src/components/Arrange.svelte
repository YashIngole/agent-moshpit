<script lang="ts">
  // Arrange, in the top bar: every open terminal in one shape, an even share for each,
  // or the one in use moved a step. There with two terminals or more. In a window too
  // narrow for one more button its items are in the menu at the end of the bar instead.
  import { tick } from 'svelte'
  import { count } from '../lib/layout'
  import { office } from '../lib/office.svelte'
  import ArrangeItems from './ArrangeItems.svelte'

  let open = $state(false)
  let root = $state<HTMLElement>()
  let width = $state(window.innerWidth)

  /** Below this the bar has no room for it; from here up to `WORD`, only for its drawing. */
  const BAR = 760
  const WORD = 1100
  const shown = $derived(count(office.layout) > 1 && width > BAR)

  $effect(() => {
    if (!shown) open = false
  })

  function close(returnFocus = false) {
    open = false
    if (returnFocus) root?.querySelector<HTMLElement>('button')?.focus()
  }

  function onDocumentPointer(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) close()
  }

  function items() {
    return [...(root?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])]
  }

  async function show(last = false) {
    open = true
    await tick()
    if (open) {
      const list = items()
      list[last ? list.length - 1 : 0]?.focus()
    }
  }

  function onKey(event: KeyboardEvent) {
    if (!open) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault()
        void show(event.key === 'ArrowUp')
      }
      return
    }
    if (event.key === 'Escape') {
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
</script>

<svelte:window bind:innerWidth={width} />
<svelte:document onpointerdown={onDocumentPointer} />

{#if shown}
  <div class="arrange" bind:this={root} onkeydown={onKey} role="presentation">
    <button
      type="button"
      class={width > WORD ? 'button quiet' : 'drawn'}
      aria-label="Arrange"
      aria-haspopup="menu"
      aria-expanded={open}
      title="Arrange the terminals: in a grid, side by side, stacked, or one of them large"
      onclick={() => (open ? close() : void show())}
    >
      {#if width > WORD}
        arrange
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2.5" y="2.5" width="11" height="11" rx="2" /><path d="M9.5 2.5v11M9.5 8h4" /></svg>
      {/if}
    </button>
    {#if open}
      <div class="sheet" role="menu" aria-label="Arrange the terminals">
        <ArrangeItems onchosen={() => close()} />
      </div>
    {/if}
  </div>
{/if}

<style>
  /* The first of the ways to act, which sit together at the end of the bar. */
  .arrange {
    position: relative;
    flex: none;
    margin-left: auto;
  }
  /* Short of room, the bar keeps only its drawing: the size of the menu button beside it. */
  .drawn {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .drawn:hover,
  .drawn[aria-expanded='true'] {
    background: var(--inset);
    color: var(--ink);
  }
  .drawn svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .sheet {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 10;
    display: grid;
    gap: var(--s-1);
    width: 312px;
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
</style>
