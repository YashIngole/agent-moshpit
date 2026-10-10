<script lang="ts">
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  let now = $state(Date.now())
  $effect(() => {
    if (!voice.active) return
    now = Date.now()
    const timer = setInterval(() => now = Date.now(), 250)
    return () => clearInterval(timer)
  })
  const target = $derived(office.agents.find(a => a.id === voice.view.agent)?.title ?? 'the selected terminal')
  const seconds = $derived(Math.max(0, Math.min(60, Math.floor((now - (voice.view.started_ms ?? now)) / 1000))))
  const elapsed = $derived(Math.max(0, Math.floor((now - (voice.view.transcribing_ms ?? now)) / 1000)))
  const error = $derived(voice.view.phase === 'error' || !!voice.problem)
  const agent = $derived(office.agents.find(a => a.id === office.focused && a.running && office.open.has(a.id)))
</script>

{#if voice.active || error || voice.view.message}
  <div class="voice-status" class:error>
    <div class="copy">
      <p role="status" aria-live="polite">
        {#if voice.problem}{voice.problem}
        {:else if voice.view.phase === 'preparing'}Starting the microphone for {target}…
        {:else if voice.view.phase === 'listening'}Listening to {target}
        {:else if voice.view.phase === 'transcribing'}Transcribing for {target} locally
        {:else}{voice.view.message}{/if}
      </p>
      {#if voice.view.phase === 'listening'}
        <div class="recording">
          <div class="meter" role="meter" aria-label="Microphone input level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={voice.view.level}><span style:width={`${voice.view.level}%`}></span></div>
          <span class="time">{seconds}s / 60s</span>
          <span class="device">{voice.view.microphone}</span>
        </div>
        <p class="hint">{seconds >= 2 && voice.view.level < 3 ? 'No sound detected. Check that your microphone is unmuted, or cancel and choose another input.' : 'Speak, then stop to insert. You review the text before pressing Enter.'}</p>
      {:else if voice.view.phase === 'transcribing'}
        <p class="hint">Microphone stopped · {elapsed}s elapsed.{elapsed >= 8 ? (voice.settings.model === 'small' ? ' Small can take a minute on CPU. Choose Base in setup for faster dictation.' : ' Still working. Longer recordings take more time.') : ''}</p>
      {/if}
    </div>
    <div class="actions">
      {#if voice.view.phase === 'listening'}<button type="button" class="button" disabled={voice.acting} onclick={() => void voice.toggle('')}>Stop and insert</button>{/if}
      {#if voice.active}<button type="button" class="button quiet" onclick={() => voice.cancel()}>Cancel voice input</button>
      {:else}
        {#if error}
          <button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel(); office.openVoice() }}>Voice setup</button>
          {#if agent && voice.ready && voice.settings.enabled}<button type="button" class="button" disabled={voice.view.busy || voice.acting} onclick={() => void voice.toggle(agent.id)}>Try again</button>{/if}
        {/if}
        <button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel() }}>Dismiss voice notice</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .voice-status { display: flex; justify-content: space-between; align-items: center; gap: var(--s-3); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--line); background: var(--panel); color: var(--work); font-size: var(--t-sm); }
  .copy { min-width: 0; display: grid; gap: var(--s-1); }
  p { overflow-wrap: anywhere; }
  .hint { color: var(--ink-2); }
  .recording { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-2); color: var(--ink-2); }
  .meter { width: 88px; height: 6px; border-radius: var(--pill); background: var(--wall); overflow: hidden; }
  .meter span { display: block; height: 100%; background: var(--work); }
  .time { font-family: var(--mono); white-space: nowrap; }
  .device { overflow-wrap: anywhere; }
  .error { color: var(--trouble-ink); background: var(--trouble-wash); }
  .actions { display: flex; flex-wrap: wrap; gap: var(--s-2); flex: none; }
  @media (max-width: 760px) { .voice-status { align-items: flex-start; flex-direction: column; gap: var(--s-2); } }
</style>
