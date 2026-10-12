<script lang="ts">
  // What Arrange offers: every open terminal in one shape, an even share for each,
  // and the terminal the keyboard is in moved a step. The items sit in the sheet under
  // the bar's Arrange button, and, in a window too narrow for that button, in the menu
  // at the end of the bar. Whichever holds them walks them with the arrow keys.
  import { tick } from 'svelte'
  import type { Shape, Side } from '../lib/layout'
  import { office } from '../lib/office.svelte'
  import { terms } from '../lib/terms'

  /** Called as a shape is chosen, so whatever holds these can close. */
  let { onchosen }: { onchosen: () => void } = $props()

  /** Each shape, and the lines that draw it inside a small room. */
  const shapes: { to: Shape; label: string; lines: string }[] = [
    { to: 'grid', label: 'In a grid', lines: 'M8 2.5v11M2.5 8h11' },
    { to: 'side', label: 'Side by side', lines: 'M6.2 2.5v11M9.8 2.5v11' },
    { to: 'stacked', label: 'Stacked', lines: 'M2.5 6.2h11M2.5 9.8h11' },
    { to: 'main-beside', label: 'This one large, the others beside it', lines: 'M9.5 2.5v11M9.5 8h4' },
    { to: 'main-below', label: 'This one large, the others below it', lines: 'M2.5 9.5h11M8 9.5v4' }
  ]
  const steps: { side: Side; word: string; key: string; arrow: string }[] = [
    { side: 'left', word: 'left', key: 'Left', arrow: 'M12.5 8h-9M7 4.5 3.500 8 7 11.5' },
    { side: 'right', word: 'right', key: 'Right', arrow: 'M3.500 8h9M9 4.5 12.500 8 9 11.5' },
    { side: 'top', word: 'up', key: 'Up', arrow: 'M8 12.500v-9M4.500 7 8 3.500 11.500 7' },
    { side: 'bottom', word: 'down', key: 'Down', arrow: 'M8 3.500v9M4.500 9 8 12.500 11.500 9' }
  ]

  /** Whose terminal the keyboard is in: "this one". An install's or an update's has no name. */
  const mine = $derived(office.open.has(office.focused) ? (office.agents.find(a => a.id === office.focused)?.title ?? '') : '')
  /** One pane has the room: the others are behind it, where nothing would be seen to move. */
  const behind = $derived(office.zoomed !== '')

  function arrange(to: Shape) {
    onchosen()
    office.arrange(to)
  }

  function even() {
    onchosen()
    office.evenOut()
    void tick().then(() => terms.get(office.focused)?.focus())
  }
</script>

{#each shapes as shape (shape.to)}
  <button type="button" role="menuitem" tabindex="-1" title={mine && shape.to.startsWith('main') ? `${mine} is the large one` : undefined} onclick={() => arrange(shape.to)}>
    <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2.5" y="2.5" width="11" height="11" rx="2" /><path d={shape.lines} /></svg>
    <span>{shape.label}</span>
  </button>
{/each}
<hr />
<button type="button" role="menuitem" tabindex="-1" class="plain" title="Every terminal an even share of the room. A double-click on an edge between two does the same." onclick={even}>
  <span>Make them even</span>
</button>
<div class="move" role="group" aria-label="Move {mine || 'this terminal'}">
  <span class="what" title={mine || undefined}>Move {mine || 'this terminal'}</span>
  {#each steps as step (step.side)}
    <button
      type="button"
      role="menuitem"
      tabindex="-1"
      class="step"
      disabled={behind}
      aria-label="Move {mine || 'this terminal'} {step.word}"
      aria-keyshortcuts="Control+Shift+Alt+Arrow{step.key}"
      title={behind ? 'Put the other terminals back to move one of them' : `Move it ${step.word} (Ctrl+Shift+Alt+${step.key})`}
      onclick={() => office.movePane(office.focused, step.side, false)}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d={step.arrow} /></svg>
    </button>
  {/each}
</div>

<style>
  /* An item as the menu at the end of the bar draws one: a name at weight 600, on one line where it fits. */
  button[role='menuitem']:not(.step) {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px var(--s-3);
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    font-size: var(--t-md);
    font-weight: 600;
    text-align: left;
  }
  button[role='menuitem']:hover:not(:disabled) {
    background: var(--inset);
  }
  /* Without a drawing of its own, its name starts where the others' do. */
  button[role='menuitem'].plain {
    padding-left: calc(var(--s-3) + 26px);
  }
  svg {
    flex: none;
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  button:not(.step) svg {
    color: var(--ink-2);
  }
  hr {
    margin: 2px var(--s-1);
    border: 0;
    border-top: 1px solid var(--line);
  }
  /* Four steps in a row, after what they move. */
  .move {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    padding: 2px var(--s-2) 2px calc(var(--s-3) + 26px);
  }
  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--t-md);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .step {
    display: grid;
    flex: none;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .step:hover:not(:disabled) {
    color: var(--ink);
  }
  .step:disabled {
    opacity: 0.45;
  }
</style>
