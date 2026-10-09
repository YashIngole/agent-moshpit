<script lang="ts">
  // A line for a moment: what just happened, and a way to undo it when there is one.
  // It is said over the floor, never over the foot of a terminal, which is where a
  // program shows its prompt and its choices: at the foot of the floor's column
  // beside the terminals, or at the top right when a narrow window has no floor.
  import { office, type Toast } from '../lib/office.svelte'

  let { toast, beside }: { toast: Toast; beside: boolean } = $props()
</script>

<div class="toast" class:beside role="status"
  onpointerenter={() => { if (toast.undo) office.pauseUndo('pointer', true) }}
  onpointerleave={() => { if (toast.undo) office.pauseUndo('pointer', false) }}
  onfocusin={() => { if (toast.undo) office.pauseUndo('focus', true) }}
  onfocusout={event => { if (toast.undo && !event.currentTarget.contains(event.relatedTarget as Node | null)) office.pauseUndo('focus', false) }}
>
  <span>{toast.text}</span>
  {#if toast.action}
    <button
      type="button"
      onclick={() => {
        toast.action!.run()
        office.toast = null
      }}>{toast.action.label}</button
    >
  {/if}
  <button type="button" class="close" aria-label={toast.undo ? 'Finish removing desks' : 'Dismiss'} title={toast.undo ? 'End their programs and finish removing these desks' : 'Dismiss'} onclick={() => office.dismissToast(toast.undo)}>
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
  </button>
</div>

<style>
  .toast {
    position: absolute;
    bottom: var(--s-4);
    left: 50%;
    z-index: 25;
    display: flex;
    align-items: center;
    gap: var(--s-3);
    max-width: min(560px, calc(100% - 2 * var(--s-4)));
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-4);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    font-size: var(--t-sm);
    line-height: 1.4;
    transform: translateX(-50%);
    animation: rise var(--settle) var(--ease) both;
  }
  @keyframes rise {
    from {
      transform: translate(-50%, 10px);
      opacity: 0;
    }
  }
  /* Terminals open: inside the floor's column, at its foot, as wide as the column allows. */
  .toast.beside {
    left: var(--s-3);
    bottom: var(--s-3);
    max-width: calc(min(var(--floor), 100% - 420px) - 2 * var(--s-3));
    transform: none;
    animation-name: rise-beside;
  }
  @keyframes rise-beside {
    from {
      transform: translateY(10px);
      opacity: 0;
    }
  }
  /* A narrow window shows no floor beside the terminals: the top right, under the bar, where no prompt is. */
  @media (max-width: 760px) {
    .toast.beside {
      top: var(--s-2);
      right: var(--s-2);
      bottom: auto;
      left: auto;
      max-width: calc(100% - 2 * var(--s-2));
      animation-name: drop;
    }
    @keyframes drop {
      from {
        transform: translateY(-10px);
        opacity: 0;
      }
    }
  }
  button {
    flex: none;
    padding: 4px var(--s-3);
    border: 1px solid var(--wall);
    border-radius: var(--r-1);
    background: transparent;
    font-family: var(--mono);
    font-size: var(--t-sm);
    font-weight: 650;
  }
  button:hover {
    background: var(--inset);
  }
  .close {
    display: grid;
    width: 26px;
    height: 26px;
    padding: 0;
    place-items: center;
    border: 0;
    color: var(--ink-2);
  }
  .close svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
  }
</style>
