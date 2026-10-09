<script lang="ts">
  // The amber band: whoever has a hand up, and the way to their terminal.
  // The question itself is on their screen, so the band only points at it.
  // It exists only while someone is waiting whose terminal is not already in front.
  import { office } from '../lib/office.svelte'
  import { elapsed } from '../lib/time'
  import { place } from '../lib/words'
  import Person from './Person.svelte'

  let chosen = $state<string | null>(null)

  // Someone whose terminal is on screen is already being attended to.
  const waiting = $derived(office.waiting.filter(a => !office.open.has(a.id) || (office.zoomed !== '' && office.zoomed !== a.id)))
  // The one picked here, else whoever has waited longest.
  const current = $derived(waiting.find(a => a.id === chosen) ?? waiting[0])
  const others = $derived(waiting.filter(a => a.id !== current?.id))
  const waited = $derived(current ? elapsed(office.now - current.since_ms) : '')
</script>

{#if current}
  <section class="band" aria-label="Needs you">
    <span class="face"><Person look={current.look} phase="needs_you" portrait /></span>
    <div class="who">
      <p class="kicker">needs you · {waited}</p>
      <h2>{current.title}</h2>
    </div>
    <p class="what">
      <code>{current.harness_tag.toLowerCase()}</code>
      {current.activity && current.activity !== 'Waiting for your answer in the terminal' ? current.activity : 'is waiting for your answer'}
      {#if place(current)}<span class="where">in {place(current)}</span>{/if}
    </p>
    <!-- Beside the terminals that are open, so nobody else's is taken from under them. -->
    <button type="button" class="answer" onclick={() => office.show(current.id, office.terminals || office.stowed.rows.length > 0)}>Open their terminal</button>
    {#if others.length > 0}
      <div class="queue">
        <span>also waiting</span>
        {#each others as agent (agent.id)}
          <button type="button" onclick={() => (chosen = agent.id)}>{agent.title}</button>
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
    max-width: 34ch;
  }
  .kicker {
    font-family: var(--mono);
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    opacity: 0.75;
  }
  h2 {
    overflow: hidden;
    font-size: var(--t-lg);
    font-weight: 650;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .what {
    flex: 1 1 260px;
    min-width: 0;
    overflow: hidden;
    font-size: var(--t-md);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .what code {
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(20, 14, 0, 0.12);
    font-family: var(--mono);
    font-size: var(--t-sm);
  }
  .where {
    margin-left: 6px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    opacity: 0.7;
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

  /* A narrow window: one line, who and the way to them. The rest is in their terminal. */
  @media (max-width: 720px) {
    .band {
      flex-wrap: nowrap;
      gap: var(--s-3);
      padding: 6px var(--s-3);
    }
    .face {
      width: 32px;
    }
    .who {
      flex: 1;
    }
    .what,
    .queue span {
      display: none;
    }
    .queue {
      flex: none;
      flex-basis: auto;
      order: 3;
      padding-top: 0;
      border-top: 0;
    }
    .queue button {
      max-width: 12ch;
    }
    .answer {
      height: 30px;
      padding: 0 10px;
    }
  }
</style>
