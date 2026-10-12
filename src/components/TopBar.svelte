<script lang="ts">
  // The bar across the top: what this is, whatever wants you and nothing else, the way
  // between the floor and the terminals, and the way to add someone.
  import { bridge } from '../lib/bridge'
  import { count as panesIn } from '../lib/layout'
  import { office } from '../lib/office.svelte'
  import { elapsed } from '../lib/time'
  import type { Phase } from '../lib/types'
  import AppMenu from './AppMenu.svelte'
  import Arrange from './Arrange.svelte'
  import Mark from './Mark.svelte'
  import Person from './Person.svelte'
  import VoiceControl from './VoiceControl.svelte'

  const count = (phase: Phase) => office.agents.filter(a => a.phase === phase).length
  /**
   * Only what wants a look is counted: someone finished, someone in trouble. Who is at
   * work can be seen on the floor, and whoever needs you has the band, or the chip below.
   */
  const counts = $derived.by(() => {
    const parts: { phase: Phase; text: string }[] = []
    if (count('done') > 0) parts.push({ phase: 'done', text: `${count('done')} done` })
    if (count('failed') > 0) parts.push({ phase: 'failed', text: `${count('failed')} in trouble` })
    return parts
  })
  // Said aloud as it changes, including who needs you: the band is not announced on its own.
  const summary = $derived.by(() => {
    const need = office.waiting.length
    return [need > 0 ? `${need} need${need === 1 ? 's' : ''} you` : '', ...counts.map(c => c.text)].filter(Boolean).join(', ')
  })

  /**
   * While terminals are open, whoever needs you is a chip here rather than a band across
   * the window, so the terminals never move under the keyboard when a hand goes up.
   */
  const first = $derived(office.terminals ? office.waiting[0] : undefined)
  const others = $derived(office.waiting.length - 1)

  function showStatuses(event: MouseEvent) {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
    office.openMenu({ x: box.left, y: box.bottom + 6, items: counts.map(part => ({ label: part.text, hint: 'Show the next one', run: () => office.findPhase([part.phase]) })) })
  }

  /** Terminals that were put away can be brought back; open ones can be put away. */
  const away = $derived(!office.terminals && office.stowed.root !== null)
  const stowedCount = $derived(panesIn(office.stowed))
</script>

<header class="bar" class:asking={first}>
  <div class="brand">
    <Mark />
    <h1>agent moshpit</h1>
  </div>
  <p class="aloud" aria-live="polite">{summary}</p>
  {#if first}
    <button
      type="button"
      class="attention"
      aria-label="{first.title} needs you{others > 0 ? `, and ${others} more` : ''}. Open their terminal"
      title="Open their terminal (Ctrl+Shift+A, again for the next)"
      onclick={() => office.nextWaiting()}
    >
      <span class="face"><Person look={first.look} phase="needs_you" portrait /></span>
      <span class="who">{first.title}</span>
      <span class="state">needs you<span class="time">{` · ${elapsed(office.now - first.since_ms)}`}</span></span>
      {#if others > 0}<span class="more">+{others}</span>{/if}
    </button>
  {/if}
  {#if counts.length > 0}
    <div class="counts">
      {#each counts as part (part.phase)}
        <button type="button" class="count" title="Show the next desk that is {part.text.replace(/^\d+ /, '')}" onclick={() => office.findPhase([part.phase])}>
          <i class="dot {part.phase}"></i>{part.text}
        </button>
      {/each}
    </div>
    <button type="button" class="compact" aria-label="{counts.map(c => c.text).join(', ')}. Show them" aria-haspopup="menu" onclick={showStatuses}>
      {#each counts as part (part.phase)}<span><i class="dot {part.phase}"></i>{part.text.replace(' in ', ' ')}</span>{/each}
    </button>
  {/if}
  {#if bridge.demo}<span class="demo">demo data</span>{/if}
  <Arrange />
  {#if office.terminals || away}
    <button type="button" class="button quiet between" title={office.terminals ? 'Hide the terminals (Ctrl+`). Their programs keep running.' : 'Bring the terminals back (Ctrl+`)'} onclick={() => office.toggleTerminals()}>
      {#if office.terminals}
        <span class="wide">hide terminals</span><span class="narrow">floor</span>
      {:else}
        terminals<span class="n">{stowedCount}</span>
      {/if}
    </button>
  {/if}
  <button type="button" class="button new" aria-label="New agent" title="New agent (n, or Ctrl+Shift+N)" onclick={() => office.openNew()}>
    + <span class="words">new agent</span>
  </button>
  <VoiceControl />
  <AppMenu />
</header>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: var(--bar);
    padding: 0 var(--s-3) 0 18px;
    border-bottom: 1px solid var(--line);
    background: var(--chrome);
  }
  .brand {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
  }
  h1 {
    font-family: var(--mono);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  /* Whoever needs you, while the terminals are open: the band's colour, in the bar's height. */
  .attention {
    display: inline-flex;
    flex: 0 1 auto;
    align-items: center;
    gap: var(--s-2);
    min-width: 0;
    height: 32px;
    margin-left: var(--s-2);
    padding: 0 10px 0 3px;
    border: 0;
    overflow: hidden;
    border-radius: var(--r-1);
    background: var(--needs);
    color: var(--needs-ink);
    font: inherit;
    animation: raise var(--settle) var(--ease) both;
    --focus: var(--ink);
  }
  @keyframes raise {
    from {
      transform: translateY(-6px);
      opacity: 0;
    }
  }
  .attention:hover {
    box-shadow: inset 0 0 0 1px rgba(20, 14, 0, 0.55);
  }
  .attention .face {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 5px;
    background: rgba(20, 14, 0, 0.14);
    overflow: hidden;
  }
  .attention .who {
    min-width: 0;
    max-width: 26ch;
    overflow: hidden;
    font-size: var(--t-sm);
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .attention .state,
  .attention .more {
    flex: none;
    font-family: var(--mono);
    font-size: var(--t-xs);
    white-space: nowrap;
  }
  .attention .more {
    padding: 0 6px;
    border: 1px solid rgba(20, 14, 0, 0.5);
    border-radius: 999px;
    font-weight: 600;
  }

  .counts {
    display: flex;
    flex: 0 1 auto;
    gap: var(--s-4);
    min-width: 0;
    margin-left: var(--s-2);
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-2);
    white-space: nowrap;
  }
  .count,
  .compact {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin: -2px -4px;
    padding: 2px 4px;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: inherit;
    font: inherit;
  }
  .count:hover,
  .compact:hover {
    background: var(--inset);
    color: var(--ink);
  }
  .compact {
    display: none;
    gap: 5px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-2);
  }
  .compact {
    gap: var(--s-3);
  }
  .compact span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  /* The menu at the end is never squeezed out: the chip and the counts give way first. */
  .bar > :global(*) {
    flex-shrink: 0;
  }
  .bar > .attention,
  .bar > .counts {
    flex-shrink: 1;
  }
  .demo {
    flex: none;
    padding: 1px var(--s-2);
    border: 1px solid var(--wall);
    border-radius: 999px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-2);
  }
  .between .n {
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--inset);
    font-size: var(--t-xs);
    color: var(--ink-2);
  }
  .narrow {
    display: none;
  }
  /* The ways to act sit together at the end of the bar. */
  .between,
  .new {
    margin-left: auto;
  }
  .between + .new,
  .bar > :global(.arrange + .between) {
    margin-left: 0;
  }

  @media (max-width: 1100px) {
    .counts,
    .demo {
      display: none;
    }
    .compact {
      display: inline-flex;
    }
    .attention .time {
      display: none;
    }
  }
  /* Whoever needs you comes before the name of the app when the bar is short of room. */
  @media (max-width: 900px) {
    .asking h1 {
      display: none;
    }
  }
  /* The floor is not beside the terminals in a window this narrow: the way back is to the floor. */
  @media (max-width: 760px) {
    .wide {
      display: none;
    }
    .narrow {
      display: inline;
    }
  }
  @media (max-width: 620px) {
    .bar {
      gap: var(--s-2);
      padding: 0 var(--s-3);
    }
    h1 {
      display: none;
    }
    .attention {
      margin-left: 0;
    }
    .attention .who {
      max-width: 14ch;
    }
  }
  /* A window kept narrow at the side of the screen: short words, so the menu always fits. */
  @media (max-width: 520px) {
    .new .words,
    .attention .who {
      display: none;
    }
    .compact {
      flex-direction: column;
      align-items: flex-start;
      gap: 0;
    }
  }
</style>
