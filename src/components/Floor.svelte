<script lang="ts">
  // The office floor: one room per project, a desk per agent.
  import { office } from '../lib/office.svelte'
  import Desk from './Desk.svelte'
  import SpareDesk from './SpareDesk.svelte'

  import type { Phase } from '../lib/types'

  const spareKey = (repo: string) => `spare:${repo}`
  /** More agents than this, and the floor draws them smaller, so a crowd fits on a screen or two. */
  const CROWD = 10
  const dense = $derived(!office.terminals && office.agents.length > CROWD)
  /** A room's empty desk to add someone at. The list and a crowd have a + in the room's sign instead. */
  const spares = $derived(!office.listed && !dense)

  /**
   * In the strip beside the terminals, whoever wants a look is gathered above the rooms,
   * however many rooms there are: needs you first, then trouble, then done; longest wait first.
   */
  const WAITING: Phase[] = ['needs_you', 'failed', 'done']
  const waiting = $derived(
    office.listed
      ? office.agents.filter(a => WAITING.includes(a.phase)).sort((a, b) => WAITING.indexOf(a.phase) - WAITING.indexOf(b.phase) || a.since_ms - b.since_ms)
      : []
  )
  const rooms = $derived(
    office.listed ? office.rooms.map(room => ({ ...room, agents: room.agents.filter(a => !WAITING.includes(a.phase)) })).filter(room => room.agents.length > 0) : office.rooms
  )

  // The floor is one stop for the Tab key, like a grid of icons: Tab lands on one desk and
  // the arrow keys walk to the rest. That desk is the last one visited, else the first.
  const home = $derived.by(() => {
    const keys = [...waiting.map(a => a.id), ...rooms.flatMap(room => [...room.agents.map(a => a.id), ...(spares ? [spareKey(room.repo)] : [])])]
    return keys.includes(office.homeDesk) ? office.homeDesk : (keys[0] ?? spareKey(''))
  })

  function visit(event: FocusEvent) {
    const key = event.target instanceof HTMLElement ? event.target.dataset.desk : undefined
    if (key !== undefined) office.homeDesk = key
  }

  /** Arrow keys walk between desks, the way they would in a grid of icons. */
  function walk(event: KeyboardEvent) {
    const keys = ['ArrowRight', 'ArrowLeft', 'ArrowDown', 'ArrowUp']
    if (!keys.includes(event.key)) return
    const here = document.activeElement
    if (!(here instanceof HTMLElement) || !here.matches('.floor button[data-desk]')) return
    const desks = [...document.querySelectorAll<HTMLElement>('.floor button[data-desk]')]
    const from = here.getBoundingClientRect()
    const cx = from.left + from.width / 2
    const cy = from.top + from.height / 2
    let best: HTMLElement | undefined
    let bestScore = Infinity
    for (const desk of desks) {
      if (desk === here) continue
      const r = desk.getBoundingClientRect()
      const dx = r.left + r.width / 2 - cx
      const dy = r.top + r.height / 2 - cy
      const ahead =
        event.key === 'ArrowRight' ? dx > 8 && Math.abs(dy) < r.height / 2
        : event.key === 'ArrowLeft' ? dx < -8 && Math.abs(dy) < r.height / 2
        : event.key === 'ArrowDown' ? dy > 8
        : dy < -8
      if (!ahead) continue
      // Prefer the nearest desk, weighting the off-axis distance so "down" stays in its column.
      const score = event.key === 'ArrowRight' || event.key === 'ArrowLeft' ? Math.abs(dx) + Math.abs(dy) * 3 : Math.abs(dy) + Math.abs(dx) * 3
      if (score < bestScore) {
        bestScore = score
        best = desk
      }
    }
    if (best) {
      event.preventDefault()
      best.focus()
      best.scrollIntoView({ block: 'nearest', inline: 'nearest' })
    }
  }

  function count(n: number) {
    return n === 1 ? '1 agent' : `${n} agents`
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main class="floor" class:dense onkeydown={walk} onfocusin={visit}>
  {#if office.rooms.length === 0}
    <section class="room first" aria-label="Your office">
      <header class="sign">
        <h2>your office</h2>
      </header>
      <div class="welcome">
        <div class="lone">
          <SpareDesk label="New agent" key={spareKey('')} home onchoose={() => office.openNew()} />
        </div>
        <div class="words">
          <p class="lead">Nobody is in yet.</p>
          <p>
            Give an agent a folder and a task, and they get a desk here with their own terminal: Claude Code, Codex, or
            whichever you use. They put a hand up when they need you, and lean back when they are done.
          </p>
          <p>Click a desk for their terminal. Hold Ctrl as you click to open another beside it, as many as you like.</p>
          <button type="button" class="button" onclick={() => office.openNew()}>+ new agent <kbd>n</kbd></button>
        </div>
      </div>
    </section>
  {:else}
    {#if waiting.length > 0}
      <section class="room waiting" aria-label="Waiting for you, {count(waiting.length)}">
        <header class="sign">
          <h2>waiting</h2>
          <span>{count(waiting.length)}</span>
        </header>
        <div class="desks">
          {#each waiting as agent (agent.id)}
            <Desk {agent} home={home === agent.id} />
          {/each}
        </div>
      </section>
    {/if}
    {#each rooms as room (room.repo)}
      <section
        class="room"
        aria-label="{room.repo}, {count(room.agents.length)}"
        style:--desks={Math.min(room.agents.length + (spares ? 1 : 0), 5)}
      >
        <header class="sign">
          <h2>{room.repo}<span class="slash">/</span></h2>
          <span>{count(room.agents.length)}</span>
          {#if !spares}
            <button type="button" class="add" aria-label="Add an agent in {room.repo}" title="Add an agent in {room.repo}" onclick={() => office.openNew(room.agents[0]?.cwd ?? '')}>+</button>
          {/if}
        </header>
        <div class="desks">
          {#each room.agents as agent (agent.id)}
            <Desk {agent} home={home === agent.id} />
          {/each}
          {#if spares}
            <SpareDesk
              label="Add an agent"
              key={spareKey(room.repo)}
              home={home === spareKey(room.repo)}
              onchoose={() => office.openNew(room.agents[0]?.cwd ?? '')}
            />
          {/if}
        </div>
      </section>
    {/each}
  {/if}
</main>

<style>
  .floor {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    align-items: flex-start;
    gap: 20px;
    min-width: 0;
    min-height: 0;
    padding: 20px;
    overflow: auto;
    /* The corridor between rooms is the dark of the building. */
    background: var(--bg);
    container: floor / inline-size;
  }

  /* A room: a graphite floor on a faint grid, edged by one line. */
  .room {
    flex: 1 1 calc(var(--desks, 2) * 204px + 2 * var(--s-4));
    min-width: 0;
    max-width: 100%;
    padding: var(--s-3) 14px 14px;
    border: 1px solid var(--wall);
    border-radius: var(--r-3);
    background:
      linear-gradient(var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      linear-gradient(90deg, var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      var(--carpet);
  }
  .sign {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
    margin-bottom: 6px;
    padding: 0 6px;
  }
  .sign h2 {
    overflow: hidden;
    font-family: var(--mono);
    font-size: var(--t-md);
    font-weight: 600;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .slash {
    color: var(--ink-3);
  }
  .add {
    display: grid;
    width: 22px;
    height: 22px;
    margin-left: auto;
    padding: 0;
    place-items: center;
    border: 1px solid var(--wall);
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
    font-family: var(--mono);
    font-size: var(--t-md);
    line-height: 1;
  }
  .add:hover {
    background: var(--inset);
    color: var(--ink);
  }
  .sign span:not(.slash) {
    flex: none;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-3);
  }

  .desks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
    gap: 6px var(--s-3);
  }

  /*
   * A crowd: smaller desks, all of one size whatever the room's width (a room of one
   * is not drawn bigger), and rooms that sit side by side sooner: two rooms of four
   * share a row in a 1280-pixel window.
   */
  .dense {
    gap: var(--s-4);
    padding: var(--s-4);
  }
  .dense .room {
    flex-basis: calc(var(--desks, 2) * 148px + 20px);
    padding: var(--s-2) 10px 10px;
  }
  .dense .desks {
    grid-template-columns: repeat(auto-fill, 140px);
    gap: 2px var(--s-2);
  }
  /* In the strip, whoever wants a look comes first, above the rooms. */
  .waiting .sign h2 {
    color: var(--ink-2);
  }

  .welcome {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-5);
    padding: var(--s-4) var(--s-2) var(--s-2);
  }
  .lone {
    flex: 0 0 200px;
  }
  .words {
    display: grid;
    flex: 1 1 260px;
    gap: var(--s-3);
    max-width: 48ch;
    justify-items: start;
  }
  .lead {
    font-size: var(--t-xl);
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .words p:not(.lead) {
    color: var(--ink-2);
    line-height: 1.55;
  }

  /* A narrow floor: one column, each desk drawn beside its nameplate. */
  @container floor (max-width: 560px) {
    .room {
      flex-basis: 100%;
      padding: var(--s-2) var(--s-2) var(--s-3);
    }
    .desks {
      grid-template-columns: minmax(0, 1fr);
      gap: 0;
    }
  }
  @container floor (max-width: 300px) {
    .room {
      padding: var(--s-1) var(--s-1) var(--s-2);
      border-radius: var(--r-2);
    }
    .sign {
      margin-bottom: 0;
      padding: var(--s-1) var(--s-2) 0;
    }
  }
  @media (max-width: 560px) {
    .floor {
      gap: var(--s-3);
      padding: var(--s-3);
    }
  }
</style>
