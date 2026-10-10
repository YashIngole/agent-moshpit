<script lang="ts">
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { shortcutLabel } from '../lib/voice'
  const agent = $derived(office.agents.find(a => a.id === office.focused && a.running && office.open.has(a.id)))
  const setup = $derived(!voice.settings.enabled || !voice.ready || !agent || !!office.panel)
  const label = $derived(voice.view.phase === 'listening' ? 'Stop recording' : voice.active ? 'Voice input in progress' : setup ? 'Set up voice input' : 'Start voice input')
</script>

  <button type="button" class="mic" class:listening={voice.view.phase === 'listening'}
    aria-label={label}
    title={voice.active ? 'Recording controls are below the bar' : setup ? 'Open voice input setup' : `Talk into ${agent?.title} (${shortcutLabel(voice.settings.shortcut)})`}
    disabled={voice.changing || voice.acting || (voice.view.phase !== 'listening' && (voice.active || voice.view.busy))}
    onclick={() => setup && !voice.active ? office.openVoice() : void voice.toggle(agent?.id ?? '')}>
    {#if voice.view.phase === 'listening'}
      <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="5" y="5" width="10" height="10" rx="1" /></svg>
    {:else}
      <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="2" width="6" height="10" rx="3" /><path d="M4 9v1a6 6 0 0 0 12 0V9M10 16v2M7 18h6" /></svg>
    {/if}
    <span>{voice.view.phase === 'listening' ? 'Stop' : voice.active ? 'Working…' : 'Voice'}</span>
  </button>

<style>
  .mic { display: inline-flex; align-items: center; gap: 5px; height: 32px; padding: 0 8px; border: 1px solid var(--line); border-radius: var(--r-1); background: transparent; color: var(--ink-2); font-family: var(--mono); font-size: var(--t-sm); }
  .mic:hover:not(:disabled) { background: var(--inset); color: var(--ink); }
  .mic:disabled { opacity: 0.55; }
  .mic.listening { color: var(--work); border-color: var(--work); }
  svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  @media (max-width: 520px) { span { display: none; } }
</style>
