<script lang="ts">
  // One desk on the floor: the person, a nameplate, and a line about what they are doing.
  // In the strip beside the terminals it is a row in a list: face, name, how they are.
  import { onMount, tick, untrack } from 'svelte'
  import { askToRemove, openDeskMenu } from '../lib/menus'
  import { office } from '../lib/office.svelte'
  import { elapsed } from '../lib/time'
  import type { Agent } from '../lib/types'
  import { describe, doing, STATUS_WORD } from '../lib/words'
  import Person from './Person.svelte'

  interface Props {
    agent: Agent
    /** Whether this desk is the floor's tab stop. Arrow keys reach the others. */
    home: boolean
  }

  let { agent, home }: Props = $props()

  /** Its terminal is on screen. */
  const open = $derived(office.open.has(agent.id))
  /** It printed something since you last looked, and is not in front of you now. */
  const unread = $derived(agent.unread && !open)
  const since = $derived(elapsed(office.now - agent.since_ms))
  /**
   * The programs do not say what kind of work they are at, so someone working changes
   * how they sit now and then: mostly typing, sometimes reading, sometimes thinking.
   */
  const tool = $derived(['edit', 'edit', 'read', 'edit', 'think'][(Math.floor(office.now / 20_000) + agent.look) % 5])
  let button = $state<HTMLButtonElement>()

  // Someone who was not here a moment ago walks to their chair, once.
  let arriving = $state(untrack(() => office.takeArrival(agent.id)))
  onMount(() => {
    if (!arriving) return
    const settled = setTimeout(() => (arriving = false), 1200)
    return () => clearTimeout(settled)
  })

  // Opening a terminal narrows the floor and the desks move, so bring this one back into view.
  $effect(() => {
    if (!open || office.focused !== agent.id) return
    void tick().then(() => {
      if (button && button.offsetParent !== null) button.scrollIntoView({ block: 'nearest' })
    })
  })

  function menuAtDesk() {
    const box = button!.getBoundingClientRect()
    openDeskMenu(agent.id, box.left + 12, box.top + Math.min(box.height, 60))
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault()
      office.toggleBeside(agent.id)
    } else if (event.key === 'F2') {
      event.preventDefault()
      office.startRename(agent.id)
    } else if (event.key === 'Delete') {
      event.preventDefault()
      askToRemove(agent.id)
    } else if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault()
      menuAtDesk()
    }
  }
</script>

<div class="seat" class:listed={office.listed} data-seat={agent.id}>
  <button
    type="button"
    class="desk {agent.phase}"
    class:open
    class:arriving
    data-desk={agent.id}
    tabindex={home ? 0 : -1}
    aria-label={describe(agent) + (unread ? ' Something new in their terminal.' : '')}
    aria-current={open ? 'true' : undefined}
    title="Open their terminal. Ctrl and a click: beside the others, or put away again. Right-click for more."
    bind:this={button}
    onclick={event => (event.ctrlKey || event.metaKey ? office.toggleBeside(agent.id) : office.show(agent.id))}
    onauxclick={event => {
      if (event.button === 1) {
        event.preventDefault()
        askToRemove(agent.id, event.clientX, event.clientY)
      }
    }}
    onmousedown={event => {
      // A middle press would otherwise start the browser's own scrolling.
      if (event.button === 1) event.preventDefault()
    }}
    oncontextmenu={event => {
      event.preventDefault()
      openDeskMenu(agent.id, event.clientX, event.clientY)
    }}
    onkeydown={onKey}
  >
    <span class="scene"><Person look={agent.look} phase={agent.phase} {tool} portrait={office.listed} /></span>
    <span class="plate">
      <span class="name">{#if unread}<i class="unread" title="Something new in their terminal since you last looked"></i>{/if}{agent.title}</span>
      <span class="status">
        <span class="chip"><i class="dot {agent.phase}"></i>{STATUS_WORD[agent.phase]}</span>
        <span class="since">{since}</span>
        <span class="tag">{agent.harness_tag}</span>
      </span>
      <span class="doing">{doing(agent)}</span>
    </span>
  </button>
  <button
    type="button"
    class="remove"
    tabindex="-1"
    aria-label="Remove {agent.title}"
    title="Take {agent.title} off the floor and end their program. It can be undone for a moment."
    onclick={event => askToRemove(agent.id, event.clientX - 260, event.clientY + 8)}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
  </button>
</div>

<style>
  .seat {
    position: relative;
    min-width: 0;
  }
  .desk {
    display: grid;
    grid-template-rows: auto auto;
    align-content: start;
    align-self: start;
    gap: var(--s-1);
    width: 100%;
    padding: var(--s-2) var(--s-2) var(--s-3);
    border: 0;
    border-radius: var(--r-2);
    background: transparent;
    text-align: left;
    transition: background var(--quick) var(--ease);
  }
  .desk:hover {
    background: rgba(255, 255, 255, 0.03);
  }
  /* Their terminal is on screen. */
  .desk.open {
    background: rgba(130, 170, 255, 0.07);
    box-shadow: inset 0 0 0 1px rgba(130, 170, 255, 0.24);
  }
  /* Found from the bar at the top: a moment's ring, then as before. */
  .desk:global(.found) {
    animation: found 1.4s var(--ease);
  }
  @keyframes found {
    0%,
    40% {
      box-shadow: inset 0 0 0 2px var(--ink);
    }
  }

  /* Taking the desk away, from the corner of it: shown when the pointer or the keyboard is on it. */
  .remove {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    width: 24px;
    height: 24px;
    padding: 0;
    place-items: center;
    border: 1px solid var(--wall);
    border-radius: var(--r-1);
    background: var(--panel);
    color: var(--ink-2);
    opacity: 0;
    transition: opacity var(--quick) var(--ease);
  }
  .seat:hover .remove,
  .seat:focus-within .remove {
    opacity: 1;
  }
  .remove:hover {
    border-color: var(--trouble-ink);
    color: var(--trouble-ink);
  }
  .remove svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .scene {
    display: block;
    padding: 0 var(--s-1);
  }

  .plate {
    display: grid;
    gap: var(--s-1);
    min-width: 0;
    padding: 0 var(--s-1);
    container: plate / inline-size;
  }
  /* Too narrow for how long, how they are and which program: how long gives way, whole, never cut to "1…". */
  @container plate (max-width: 210px) {
    .since {
      display: none;
    }
  }
  .name {
    overflow: hidden;
    font-size: var(--t-md);
    font-weight: 600;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Something new in their terminal: a small mark in ink, not a status colour. */
  .unread {
    display: inline-block;
    width: 7px;
    height: 7px;
    margin: 0 6px 1px 0;
    border-radius: 50%;
    background: var(--ink);
    vertical-align: middle;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--ink-2);
  }
  .chip {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 7px;
    text-transform: lowercase;
    white-space: nowrap;
  }
  .needs_you .chip {
    color: var(--needs);
    font-weight: 600;
  }
  .failed .chip {
    color: var(--trouble-ink);
  }
  .since {
    flex: none;
    color: var(--ink-3);
    white-space: nowrap;
  }
  /* Which program this person is: a word, never a colour. It gives way before how they are. */
  .tag {
    flex: 0 1 auto;
    min-width: 0;
    margin-left: auto;
    padding: 0 6px;
    overflow: hidden;
    border: 1px solid var(--wall);
    border-radius: 5px;
    font-size: 11px;
    line-height: 1.6;
    color: var(--ink-2);
    text-overflow: ellipsis;
    text-transform: lowercase;
    white-space: nowrap;
  }
  .doing {
    display: -webkit-box;
    overflow: hidden;
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--ink-3);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow-wrap: anywhere;
  }
  .failed .doing {
    color: var(--trouble-ink);
  }

  @container floor (max-width: 560px) {
    .desk {
      grid-template-columns: 116px minmax(0, 1fr);
      grid-template-rows: auto;
      align-items: center;
      gap: var(--s-2);
      padding: var(--s-2);
    }
    .scene {
      padding: 0;
    }
  }

  /* A narrow floor that is not yet a list: who is here, how they stand, and which program. */
  @container floor (max-width: 340px) {
    .desk {
      grid-template-columns: 84px minmax(0, 1fr);
      padding: var(--s-1) 6px;
    }
    .doing {
      display: none;
    }
  }

  /* A crowd on the floor: the line about what they are doing gives way, and the desk tightens. */
  :global(.floor.dense) .desk {
    padding: var(--s-1) var(--s-1) var(--s-2);
  }
  :global(.floor.dense) .doing {
    display: none;
  }
  :global(.floor.dense) .name {
    font-size: var(--t-sm);
  }
  :global(.floor.dense) .plate {
    padding: 0 2px;
  }
  :global(.floor.dense) .status {
    gap: 5px;
    font-size: 11px;
  }
  :global(.floor.dense) .tag {
    padding: 0 4px;
    font-size: 10.5px;
    line-height: 1.5;
  }

  /* The strip beside the terminals is a list of names, like a terminal's tabs. */
  .listed .desk {
    grid-template-columns: 30px minmax(0, 1fr);
    grid-template-rows: auto;
    align-items: center;
    gap: var(--s-2);
    padding: 6px 30px 6px 6px;
  }
  .listed .scene {
    width: 30px;
    padding: 0;
    border-radius: var(--r-1);
    background: var(--inset);
    overflow: hidden;
  }
  .listed .plate {
    gap: 2px;
    padding: 0;
  }
  .listed .name {
    display: -webkit-box;
    font-size: var(--t-sm);
    white-space: normal;
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .listed .status {
    font-size: 11px;
  }
  .listed .doing {
    display: none;
  }
  /* Which program, on every row of the list, as a terminal's tab would say it. */
  .listed .tag {
    padding: 0 4px;
    font-size: 10.5px;
    line-height: 1.5;
  }
  .listed .remove {
    top: 50%;
    right: 4px;
    width: 22px;
    height: 22px;
    margin-top: -11px;
    border-color: transparent;
    background: transparent;
  }
</style>
