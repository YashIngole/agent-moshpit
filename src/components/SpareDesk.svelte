<script lang="ts">
  // An empty place at the end of a room. Choosing it seats a new agent.
  interface Props {
    label: string
    /** Names this desk among the floor's desks, for the arrow keys. */
    key: string
    /** Whether this desk is the floor's tab stop. */
    home: boolean
    onchoose: () => void
  }

  let { label, key, home, onchoose }: Props = $props()
</script>

<button type="button" class="spare" data-desk={key} tabindex={home ? 0 : -1} onclick={onchoose}>
  <span class="plus" aria-hidden="true">
    <svg viewBox="0 0 16 16"><path d="M8 3v10M3 8h10" /></svg>
  </span>
  <span class="words">{label}</span>
</button>

<style>
  .spare {
    display: grid;
    gap: var(--s-2);
    place-content: center;
    justify-items: center;
    align-self: stretch;
    width: 100%;
    min-height: 120px;
    padding: var(--s-3);
    border: 1px dashed var(--wall);
    border-radius: var(--r-2);
    background: transparent;
    color: var(--ink-3);
    transition:
      background var(--quick) var(--ease),
      color var(--quick) var(--ease),
      border-color var(--quick) var(--ease);
  }
  .spare:hover,
  .spare:focus-visible {
    border-color: var(--field-line);
    background: rgba(255, 255, 255, 0.025);
    color: var(--ink);
  }
  .plus {
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1px solid var(--wall);
    border-radius: 9px;
  }
  .plus svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
  }
  .words {
    font-family: var(--mono);
    font-size: var(--t-sm);
    text-transform: lowercase;
  }

  @container floor (max-width: 560px) {
    .spare {
      grid-auto-flow: column;
      justify-content: start;
      gap: var(--s-3);
      min-height: 0;
      padding: var(--s-2) var(--s-3);
    }
  }
</style>
