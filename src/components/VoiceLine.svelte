<script lang="ts">
  // A recording for the field above it: what the microphone is doing, the words still being
  // decided, and how to end it. It stands in the form, under the field its words go into;
  // the card over the terminals is for recordings into a terminal.
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { engineName, shortcutLabel } from '../lib/voice'

  /** `back` returns the keyboard to the field after a button here has been used. */
  let { back }: { back: () => void } = $props()

  let now = $state(Date.now())
  $effect(() => {
    if (!voice.active) return
    now = Date.now()
    const timer = setInterval(() => (now = Date.now()), 250)
    return () => clearInterval(timer)
  })
  const phase = $derived(voice.view.phase)
  const seconds = $derived(Math.max(0, Math.floor((now - (voice.view.started_ms ?? now)) / 1000)))
  const error = $derived(phase === 'error' || !!voice.problem)
  /** "Added to the task." is said aloud only: the words are in the field for anyone looking. A
   *  note that asks for a second look is shown. */
  const plain = $derived(voice.notice === 'Added to the task.')
  const shown = $derived(voice.active || error || (!!voice.notice && !plain))
  const shortcut = $derived(voice.settings.shortcut === 'none' ? '' : shortcutLabel(voice.settings.shortcut))

  function stop() {
    back()
    void voice.finish()
  }
  function close() {
    if (voice.active) voice.cancel()
    else if (error) {
      voice.problem = ''
      voice.cancel()
    } else voice.dismissNotice()
    back()
  }
</script>

<p class="aloud" role="status">{plain ? voice.notice : ''}</p>
{#if shown}
  <div class="voice-line" class:error class:listening={phase === 'listening'}>
    <div class="head">
      <p class="state" role={error ? 'alert' : 'status'}>
        {#if voice.problem}{voice.problem}
        {:else if phase === 'preparing'}<span class="dot"></span>{voice.view.detail || 'Starting the microphone…'}
        {:else if phase === 'listening'}<span class="dot"></span>Listening{#if voice.view.engine !== 'local'}<span class="via">· {engineName(voice.view.engine)}</span>{/if}
        {:else if phase === 'transcribing'}<span class="spin"></span>Adding the last words…
        {:else if error}{voice.view.message}
        {:else}{voice.notice}{/if}
      </p>
      {#if phase === 'listening'}<span class="time">{Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, '0')}</span>{/if}
      <div class="actions">
        {#if phase === 'listening'}
          <button type="button" class="button" aria-label="Stop and add the last words" title={shortcut ? `Stop and add the last words (${shortcut})` : 'Stop and add the last words'} disabled={voice.acting} onmousedown={event => event.preventDefault()} onclick={stop}>Stop</button>
        {/if}
        {#if error}
          <button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel(); office.openVoice() }}>Voice setup</button>
          {#if voice.ready && voice.settings.enabled}
            <button type="button" class="button" disabled={voice.view.busy || voice.acting} onclick={() => { back(); void voice.toggle('') }}>Try again</button>
          {/if}
        {/if}
        <button type="button" class="close" aria-label={voice.active ? 'Cancel voice input' : 'Dismiss voice notice'} title={voice.active ? 'Cancel: stop listening; words not yet added are dropped' : 'Dismiss'} onmousedown={event => event.preventDefault()} onclick={close}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
        </button>
      </div>
    </div>
    {#if phase === 'listening' || phase === 'transcribing'}
      <!-- The phrase being spoken; phrases already finished are in the field. -->
      <p class="words" class:waiting={!voice.view.partial}>{voice.view.partial || (phase === 'listening' ? (voice.view.detail ? `${voice.view.detail} Start speaking; nothing is lost.` : seconds >= 2 && voice.view.level < 3 ? 'No sound detected. Check that your microphone is unmuted, or cancel and choose another input.' : 'Speak. Your words are added to the task as they settle.') : '')}</p>
    {/if}
    {#if phase === 'listening'}
      <div class="meter" role="meter" aria-label="Microphone input level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={voice.view.level}><span style:transform={`scaleX(${voice.view.level / 100})`}></span></div>
    {/if}
  </div>
{/if}

<style>
  .voice-line {
    display: grid;
    gap: var(--s-1);
    margin-top: var(--s-2);
    padding: var(--s-1) var(--s-1) var(--s-2) var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-1);
    background: var(--inset);
    font-size: var(--t-sm);
    line-height: 1.4;
  }
  .listening { border-color: var(--work); }
  .error { border-color: var(--trouble-ink); background: var(--trouble-wash); }
  .head { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-2); min-height: 30px; }
  p { overflow-wrap: anywhere; }
  .state { display: flex; flex: 1 1 8em; min-width: 0; align-items: center; gap: var(--s-2); color: var(--work); font-weight: 650; }
  /* What went wrong takes the line to itself; the ways out go under it. */
  .error .state { display: block; flex-basis: 100%; color: var(--trouble-ink); }
  .via { color: var(--ink-2); font-weight: 400; white-space: nowrap; }
  .time { flex: none; font-family: var(--mono); color: var(--ink-2); }
  .actions { display: flex; flex: none; align-items: center; gap: var(--s-1); margin-left: auto; }
  .actions .button { min-height: 28px; padding: 0 var(--s-3); }
  .close { display: grid; width: 28px; height: 28px; place-items: center; border: 0; border-radius: var(--r-1); background: transparent; color: var(--ink-2); }
  .close:hover { background: var(--panel); color: var(--ink); }
  .close svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; }
  /* The newest words stay in sight: a long phrase is cut at its start, not its end. */
  .words { display: flex; flex-direction: column-reverse; max-height: 2.8em; padding-right: var(--s-2); overflow: hidden; color: var(--ink); }
  .words.waiting { color: var(--ink-2); }
  .meter { width: 72px; height: 4px; border-radius: var(--pill); background: var(--wall); overflow: hidden; }
  .meter span { display: block; height: 100%; background: var(--work); transform-origin: left; transition: transform 80ms linear; }
  .dot { width: 9px; height: 9px; flex: none; border-radius: 50%; background: var(--work); animation: pulse 1.2s ease-in-out infinite; }
  .spin { width: 11px; height: 11px; flex: none; border: 2px solid var(--wall); border-top-color: var(--work); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .dot, .spin { animation: none; } }
</style>
