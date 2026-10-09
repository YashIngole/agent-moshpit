<script lang="ts">
  // The bar across the top: what this is, the state of the room in one line, the
  // way between the floor and the terminals, and the way to add someone.
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import type { Phase } from '../lib/types'
  import AppMenu from './AppMenu.svelte'
  import Mark from './Mark.svelte'
  import VoiceControl from './VoiceControl.svelte'

  const counts = $derived.by(() => {
    const count = (...phases: Phase[]) => office.agents.filter(a => phases.includes(a.phase)).length
    const parts: { phase: Phase; text: string }[] = []
    const need = count('needs_you')
    if (need > 0) parts.push({ phase: 'needs_you', text: `${need} need${need === 1 ? 's' : ''} you` })
    // Someone still starting up is not at work yet: counted when they are.
    if (count('working') > 0) parts.push({ phase: 'working', text: `${count('working')} working` })
    if (count('quiet') > 0) parts.push({ phase: 'quiet', text: `${count('quiet')} quiet` })
    if (count('done') > 0) parts.push({ phase: 'done', text: `${count('done')} done` })
    if (count('failed') > 0) parts.push({ phase: 'failed', text: `${count('failed')} in trouble` })
    return parts
  })
  const summary = $derived(counts.length > 0 ? counts.map(c => c.text).join(', ') : office.agents.length === 0 ? '' : 'All quiet')

  /** Which states a count in the bar stands for: a click on it finds the next desk in one of them. */
  const PHASES: Record<string, Phase[]> = { needs_you: ['needs_you'], working: ['working'], quiet: ['quiet'], done: ['done'], failed: ['failed'] }
  const compact = $derived(counts.filter(c => c.phase === 'needs_you' || c.phase === 'failed'))
  function showStatuses(event: MouseEvent) {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
    office.openMenu({ x: box.left, y: box.bottom + 6, items: counts.map(part => ({ label: part.text, hint: 'Show the next agent', run: () => office.findPhase(PHASES[part.phase] ?? []) })) })
  }

  /** Terminals that were put away can be brought back; open ones can be put away. */
  const away = $derived(!office.terminals && office.stowed.rows.length > 0)
</script>

<header class="bar">
  <div class="brand">
    <Mark />
    <h1>agent moshpit</h1>
  </div>
  <p class="summary" aria-live="polite">
    <span class="aloud">{summary}</span>
    {#each counts as part (part.phase)}
      <button type="button" class="count" aria-label="Show the next agent: {part.text}" title="Show the next desk that is {part.text.replace(/^\d+ /, '')}" onclick={() => office.findPhase(PHASES[part.phase] ?? [])}>
        <i class="dot {part.phase}"></i>{part.text}
      </button>
    {:else}
      {#if summary}<span class="count" aria-hidden="true">{summary.toLowerCase()}</span>{/if}
    {/each}
  </p>
  {#if counts.length > 0}
    <button type="button" class="compact" aria-label="{summary}. Show agent statuses" aria-haspopup="menu" onclick={showStatuses}>
      {#each compact.length > 0 ? compact : counts.slice(0, 1) as part (part.phase)}
        <span><i class="dot {part.phase}"></i>{part.text}</span>
      {/each}
    </button>
  {/if}
  {#if bridge.demo}<span class="demo">demo data</span>{/if}
  {#if office.terminals || away}
    <button type="button" class="button quiet between" title={office.terminals ? 'Back to the floor (Ctrl+`)' : 'The terminals (Ctrl+`)'} onclick={() => office.toggleTerminals()}>
      <span class="words">{office.terminals ? 'back to the floor' : 'terminals'}</span>
      <span class="short">{office.terminals ? 'floor' : 'terminals'}</span>
      <kbd>ctrl `</kbd>
    </button>
  {/if}
  <button type="button" class="button new" aria-label="New agent" title="New agent (n, or Ctrl+Shift+N)" onclick={() => office.openNew()}>
    + <span class="words">new agent</span> <kbd>n</kbd>
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
  .summary {
    display: flex;
    flex: 1;
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
    margin-right: auto;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-2);
  }
  .compact span { display: inline-flex; align-items: center; gap: 5px; }
  .short {
    display: none;
  }
  /* The menu at the end is never squeezed out: the summary gives way first. */
  .bar > :global(*) {
    flex-shrink: 0;
  }
  .bar > .summary {
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
  .button kbd {
    margin-left: 2px;
  }
  .new { margin-left: auto; }

  @media (max-width: 860px) {
    .button kbd {
      display: none;
    }
  }
  @media (max-width: 1180px) {
    .summary, .demo { display: none; }
    .compact { display: inline-flex; }
  }
  @media (max-width: 620px) {
    .bar {
      gap: var(--s-2);
      padding: 0 var(--s-3);
    }
    h1,
    .summary,
    .demo {
      display: none;
    }
    .brand {
      flex: none;
      flex-shrink: 1;
      min-width: 0;
    }
  }
  /* A window kept narrow at the side of the screen: short words, so the menu always fits. */
  @media (max-width: 520px) {
    .compact { flex-direction: column; align-items: flex-start; gap: 0; }
    .between .words,
    .new .words {
      display: none;
    }
    .between .short {
      display: inline;
    }
  }
</style>
