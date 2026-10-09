<script lang="ts">
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { shortcutLabel } from '../lib/voice'
  const agent = $derived(office.agents.find(a => a.id === office.focused && a.running && office.open.has(a.id)))
</script>

{#if voice.settings.enabled}
  <button type="button" class="mic" class:listening={voice.view.phase === 'listening'}
    aria-label={voice.view.phase === 'listening' ? 'Stop recording' : 'Start voice input'}
    title={voice.active ? 'Recording controls are below the bar' : !voice.ready ? 'Download a model in More → Voice input' : !agent ? 'Select a live terminal to talk into' : `Talk into ${agent.title} (${shortcutLabel(voice.settings.shortcut)})`}
    disabled={voice.changing || (voice.view.phase !== 'listening' && (voice.view.busy || !voice.ready || !agent))}
    onclick={() => void voice.toggle(agent?.id ?? '')}>
    <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="2" width="6" height="10" rx="3" /><path d="M4 9v1a6 6 0 0 0 12 0V9M10 16v2M7 18h6" /></svg>
    <span>mic</span>
  </button>
{/if}

<style>
  .mic { display: inline-flex; align-items: center; gap: 5px; height: 32px; padding: 0 8px; border: 1px solid var(--line); border-radius: var(--r-1); background: transparent; color: var(--ink-2); font-family: var(--mono); font-size: var(--t-sm); }
  .mic:hover:not(:disabled) { background: var(--inset); color: var(--ink); }
  .mic.listening { color: var(--work); border-color: var(--work); }
  svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  @media (max-width: 520px) { span { display: none; } }
</style>
