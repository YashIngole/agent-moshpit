<script lang="ts">
  // The amber band: whoever has a hand up, and the way to their terminal.
  // The question itself is on their screen, so the band only points at it.
  // It stays until the program confirms that the question has been answered.
  // While terminals are open the bar carries it as a chip instead (TopBar), so they
  // never move under the keyboard when a hand goes up.
  import { office } from '../lib/office.svelte'
  import { elapsed } from '../lib/time'
  import { doing, place } from '../lib/words'
  import Person from './Person.svelte'

  const waiting = $derived(office.terminals ? [] : office.waiting)
  // Whoever has waited longest.
  const current = $derived(waiting[0])
  const others = $derived(waiting.slice(1))
  const waited = $derived(current ? elapsed(office.now - current.since_ms) : '')
  /** What they asked, when the program said; nothing when it only said that it is waiting. */
  const asked = $derived(current ? doing(current) : '')
  const open = (id: string) => office.show(id, office.terminals || office.stowed.root !== null)
</script>

{#if current}
  <section class="band" aria-label="Needs you">
    <span class="face"><Person look={current.look} phase="needs_you" portrait /></span>
    <div class="who">
      <h2 title={current.title}>{current.title}</h2>
      <p class="meta">
        <strong>needs you</strong> · {waited} · {current.harness_tag.toLowerCase()}{#if place(current)}<span class="where">{` · ${place(current)}`}</span>{/if}
      </p>
    </div>
    <p class="what" title={asked}>{asked}</p>
    <!-- Beside the terminals that are open, so nobody else's is taken from under them. -->
    <button type="button" class="answer" title="Open their terminal (Ctrl+Shift+A)" onclick={() => open(current.id)}>Open their terminal</button>
    {#if others.length > 0}
      <div class="queue">
        <span>also waiting</span>
        {#each others as agent (agent.id)}
          <button type="button" title="Open {agent.title}'s terminal" onclick={() => open(agent.id)}>{agent.title}</button>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .band {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2) var(--s-4);
    padding: 10px 14px 10px 18px;
    background: var(--needs);
    color: var(--needs-ink);
    animation: drop var(--settle) var(--ease) both;
    --focus: var(--needs-ink);
  }
  @keyframes drop {
    from {
      transform: translateY(-14px);
      opacity: 0;
    }
  }

  .face {
    flex: none;
    width: 44px;
    border-radius: var(--r-2);
    background: rgba(20, 14, 0, 0.14);
    overflow: hidden;
  }
  .who {
    display: grid;
    gap: 2px;
    min-width: 0;
    max-width: 52ch;
  }
  h2 {
    overflow: hidden;
    font-size: var(--t-lg);
    font-weight: 650;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    overflow: hidden;
    font-family: var(--mono);
    font-size: var(--t-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta strong {
    font-weight: 600;
  }
  .where {
    opacity: 0.75;
  }
  /* What they asked, when the program said it. */
  .what {
    flex: 1 1 200px;
    min-width: 0;
    overflow: hidden;
    font-size: var(--t-md);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .answer {
    flex: none;
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-1);
    background: var(--needs-ink);
    color: var(--needs);
    font-family: var(--mono);
    font-size: var(--t-sm);
    font-weight: 600;
    text-transform: lowercase;
  }
  .answer:hover {
    background: #2a1f04;
  }

  .queue {
    display: flex;
    flex-basis: 100%;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2);
    padding-top: var(--s-2);
    border-top: 1px solid rgba(20, 14, 0, 0.22);
    font-size: var(--t-sm);
  }
  .queue span {
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .queue button {
    max-width: 24ch;
    padding: 2px var(--s-3);
    overflow: hidden;
    border: 1px solid rgba(20, 14, 0, 0.55);
    border-radius: 999px;
    background: transparent;
    color: var(--needs-ink);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .queue button:hover {
    background: rgba(20, 14, 0, 0.1);
  }

  /* Keep identity and action together; the other waiting desks get their own row. */
  @media (max-width: 720px) {
    .band {
      flex-wrap: wrap;
      gap: var(--s-3);
      padding: 6px var(--s-3);
    }
    .face {
      width: 32px;
    }
    .who {
      flex: 1 1 100px;
    }
    .what {
      display: none;
    }
    .queue {
      flex: 1 1 100%;
      min-width: 0;
    }
    .queue button {
      max-width: min(24ch, 100%);
    }
    .answer {
      height: 30px;
      padding: 0 10px;
    }
  }
  @media (max-width: 420px) {
    .face { display: none; }
    .who { max-width: none; }
    .answer { padding: 0 var(--s-2); }
  }
</style>
