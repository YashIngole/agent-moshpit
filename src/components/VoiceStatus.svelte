<script lang="ts">
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  let now = $state(Date.now())
  $effect(() => {
    if (voice.view.phase !== 'listening') return
    now = Date.now()
    const timer = setInterval(() => now = Date.now(), 250)
    return () => clearInterval(timer)
  })
  const target = $derived(office.agents.find(a => a.id === voice.view.agent)?.title ?? 'the selected terminal')
  const seconds = $derived(Math.max(0, Math.min(60, Math.floor((now - (voice.view.started_ms ?? now)) / 1000))))
</script>

{#if voice.active || voice.view.phase === 'error' || voice.view.message || voice.problem}
  <div class="voice-status" class:error={voice.view.phase === 'error' || !!voice.problem}>
    <p role="status" aria-live="polite">
      {#if voice.problem}{voice.problem}
      {:else if voice.view.phase === 'preparing'}Starting the microphone for {target}…
      {:else if voice.view.phase === 'listening'}Listening to {target} · {seconds}s / 60s
      {:else if voice.view.phase === 'transcribing'}Transcribing for {target} locally. Microphone stopped.
      {:else}{voice.view.message}{/if}
    </p>
    <div class="actions">
      {#if voice.view.phase === 'listening'}<button type="button" class="button quiet" onclick={() => void voice.toggle('')}>Stop and insert</button>{/if}
      {#if voice.active}<button type="button" class="button quiet" onclick={() => voice.cancel()}>Cancel voice input</button>
      {:else}<button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel() }}>Dismiss voice notice</button>{/if}
    </div>
  </div>
{/if}

<style>
  .voice-status { display: flex; justify-content: space-between; align-items: center; gap: var(--s-3); padding: var(--s-2) var(--s-3) var(--s-2) var(--s-4); border-bottom: 1px solid var(--line); background: var(--panel); color: var(--work); font-size: var(--t-sm); }
  p { overflow-wrap: anywhere; }
  .error { color: var(--trouble-ink); background: var(--trouble-wash); }
  .actions { display: flex; gap: var(--s-2); flex: none; }
  @media (max-width: 620px) { .voice-status { align-items: flex-start; flex-direction: column; gap: var(--s-1); } }
</style>
