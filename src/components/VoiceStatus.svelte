<script lang="ts">
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { engineName, shortcutLabel } from '../lib/voice'
  let now = $state(Date.now())
  $effect(() => {
    if (!voice.active) return
    now = Date.now()
    const timer = setInterval(() => now = Date.now(), 250)
    return () => clearInterval(timer)
  })
  const target = $derived(office.agents.find(a => a.id === voice.view.agent)?.title ?? 'the selected terminal')
  const seconds = $derived(Math.max(0, Math.floor((now - (voice.view.started_ms ?? now)) / 1000)))
  const error = $derived(voice.view.phase === 'error' || !!voice.problem)
  const agent = $derived(office.agents.find(a => a.id === office.voicePane && a.running))
  const shortcut = $derived(voice.settings.shortcut === 'none' ? '' : shortcutLabel(voice.settings.shortcut))
</script>

<!-- Over the work, never in the column: showing it must not resize the terminals under it.
     A recording for a field in a form is shown in that form, under the field. -->
{#if !voice.form && !voice.view.field && (voice.active || error || voice.notice)}
  <div class="voice-status" class:error class:listening={voice.view.phase === 'listening'}>
    <div class="head">
      <p class="state" role="status" aria-live="polite">
        {#if voice.problem}{voice.problem}
        {:else if voice.view.phase === 'preparing'}<span class="dot"></span>{voice.view.detail || `Starting the microphone for ${target}…`}
        {:else if voice.view.phase === 'listening'}<span class="dot"></span>Listening to {target}{#if voice.view.engine !== 'local'}<span class="via">· {engineName(voice.view.engine)}</span>{/if}
        {:else if voice.view.phase === 'transcribing'}<span class="spin"></span>Typing the last words into {target}…
        {:else if error}{voice.view.message}
        {:else}{voice.notice}{/if}
      </p>
      {#if voice.view.phase === 'listening'}<span class="time">{Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, '0')}</span>{/if}
      <div class="actions">
        {#if voice.view.phase === 'listening'}<button type="button" class="button" aria-label="Stop and insert" title="Stop and type the last words" disabled={voice.acting} onclick={() => void voice.finish()}>Stop</button>{/if}
        {#if error}
          <button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel(); office.openVoice() }}>Voice setup</button>
          {#if agent && voice.ready && voice.settings.enabled}<button type="button" class="button" disabled={voice.view.busy || voice.acting} onclick={() => void voice.toggle(agent.id)}>Try again</button>{/if}
        {/if}
        <button type="button" class="close" aria-label={voice.active ? 'Cancel voice input' : 'Dismiss voice notice'} title={voice.active ? 'Cancel: stop listening; words not yet typed are dropped' : 'Dismiss'}
          onclick={() => { if (voice.active) voice.cancel(); else if (error) { voice.problem = ''; voice.cancel() } else voice.dismissNotice() }}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
        </button>
      </div>
    </div>
    {#if voice.view.phase === 'listening' || voice.view.phase === 'transcribing'}
      <!-- The phrase being spoken; phrases already finished are in the terminal. -->
      <p class="words" class:waiting={!voice.view.partial}>{voice.view.partial || (voice.view.phase === 'listening' ? (voice.view.detail ? `${voice.view.detail} Start speaking; nothing is lost.` : seconds >= 2 && voice.view.level < 3 ? 'No sound detected. Check that your microphone is unmuted, or cancel and choose another input.' : 'Speak. Your words are typed into the terminal as they settle.') : '')}</p>
    {/if}
    {#if voice.view.phase === 'listening'}
      <div class="recording">
        <div class="meter" role="meter" aria-label="Microphone input level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={voice.view.level}><span style:transform={`scaleX(${voice.view.level / 100})`}></span></div>
        <span class="device">{voice.view.microphone}</span>
        <span class="hint">{shortcut ? `Let go of ${shortcut}, or press it again, to finish.` : 'Stop to finish.'} You press Enter to send.</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Centred over the terminals when they are open, over the floor when they are not. */
  .voice-status {
    --across: 100%;
    --left: 0px;
    position: absolute;
    top: var(--s-2);
    left: calc(var(--left) + var(--across) / 2);
    z-index: 30;
    display: grid;
    gap: var(--s-2);
    width: min(640px, calc(var(--across) - 2 * var(--s-3)));
    padding: var(--s-2) var(--s-2) var(--s-3) var(--s-4);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    color: var(--ink);
    font-size: var(--t-sm);
    line-height: 1.4;
    transform: translateX(-50%);
  }
  :global(.terminals) .voice-status {
    --left: min(var(--floor), 100% - 420px);
    --across: calc(100% - min(var(--floor), 100% - 420px));
  }
  .listening { border-color: var(--work); }
  .head { display: flex; align-items: center; gap: var(--s-3); min-height: 32px; }
  p { overflow-wrap: anywhere; }
  .state { display: flex; flex: 1; min-width: 0; align-items: center; gap: var(--s-2); color: var(--work); font-weight: 650; }
  .error .state { color: var(--trouble-ink); }
  .via { color: var(--ink-2); font-weight: 400; white-space: nowrap; }
  .time { flex: none; font-family: var(--mono); color: var(--ink-2); white-space: nowrap; }
  .words { padding-right: var(--s-3); font-size: var(--t-lg); line-height: 1.35; color: var(--ink); max-height: 4.1em; overflow: hidden; display: flex; flex-direction: column-reverse; }
  .words.waiting { font-size: var(--t-sm); color: var(--ink-2); }
  .recording { display: flex; flex-wrap: wrap; align-items: center; column-gap: var(--s-3); row-gap: var(--s-1); padding-right: var(--s-3); color: var(--ink-2); }
  .meter { width: 72px; height: 6px; flex: none; border-radius: var(--pill); background: var(--wall); overflow: hidden; }
  .meter span { display: block; height: 100%; background: var(--work); transform-origin: left; transition: transform 80ms linear; }
  .hint, .device { color: var(--ink-2); overflow-wrap: anywhere; }
  .dot { width: 9px; height: 9px; flex: none; border-radius: 50%; background: var(--work); animation: pulse 1.2s ease-in-out infinite; }
  .spin { width: 11px; height: 11px; flex: none; border: 2px solid var(--wall); border-top-color: var(--work); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .dot, .spin { animation: none; } }
  .error { background: var(--trouble-wash); border-color: var(--trouble-ink); }
  .actions { display: flex; flex-wrap: wrap; justify-content: flex-end; align-items: center; gap: var(--s-2); flex: none; }
  .actions .button { min-height: 30px; padding: 0 var(--s-3); }
  .close { display: grid; width: 30px; height: 30px; place-items: center; border: 0; border-radius: var(--r-1); background: transparent; color: var(--ink-2); }
  .close:hover { background: var(--inset); color: var(--ink); }
  .close svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; }
  @media (max-width: 760px) {
    :global(.terminals) .voice-status { --left: 0px; --across: 100%; }
    .head { flex-wrap: wrap; }
  }
</style>
