<script lang="ts">
  // A recording into a terminal: that it is listening, the words still being decided, and
  // the ways to end it. It sits in the terminal its words go into, so it need not say where
  // (`docked`, see Panes); when that terminal is not in front it floats over the work and
  // names it instead. A recording for a field in a form is shown in that form, under the field.
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { engineName, isPhrased, shortcutLabel } from '../lib/voice'

  /** `below`: under the find bar, which has the same corner of the terminal. */
  let { docked = false, below = false }: { docked?: boolean; below?: boolean } = $props()

  let now = $state(Date.now())
  $effect(() => {
    if (!voice.active) return
    now = Date.now()
    const timer = setInterval(() => now = Date.now(), 250)
    return () => clearInterval(timer)
  })
  const phase = $derived(voice.view.phase)
  const into = $derived(office.agents.find(a => a.id === voice.view.agent))
  const target = $derived(into?.title ?? 'the selected terminal')
  const seconds = $derived(Math.max(0, Math.floor((now - (voice.view.started_ms ?? now)) / 1000)))
  const error = $derived(phase === 'error' || !!voice.problem)
  /** The terminal Try again records into: the one in front now. */
  const again = $derived(office.agents.find(a => a.id === office.voicePane && a.running))
  const shortcut = $derived(voice.settings.shortcut === 'none' ? '' : shortcutLabel(voice.settings.shortcut))
  /** The microphone has been open a while and the meter has not moved. */
  const silent = $derived(phase === 'listening' && !voice.view.partial && !voice.view.detail && seconds >= 2 && voice.view.level < 3)
  /** That the words went in is plain to see in their terminal; said here only when it is not in front. */
  const noted = $derived(!!voice.notice && !(voice.plain && docked))
  // With a form open, what went wrong and what was added are the form's to say.
  const shown = $derived(!voice.view.field && (voice.active || (!voice.form && (error || noted))))

  function close() {
    if (voice.active) voice.cancel()
    else if (error) {
      voice.problem = ''
      voice.cancel()
    } else voice.dismissNotice()
  }
</script>

{#if !voice.view.field && !voice.form}<p class="aloud" role="status">{voice.plain && docked ? voice.notice : ''}</p>{/if}
{#if shown}
  <div class="voice-status" class:docked class:below class:error class:listening={phase === 'listening'} role="group" aria-label="Voice input">
    <div class="head">
      {#if phase === 'listening'}
        <div class="meter" role="meter" aria-label="Microphone input level" aria-valuemin="0" aria-valuemax="100" aria-valuenow={voice.view.level} title={voice.view.microphone}><span style:transform={`scaleX(${voice.view.level / 100})`}></span></div>
      {:else if voice.active}<span class="spin"></span>{/if}
      <p class="state" role={error ? 'alert' : 'status'}>
        {#if voice.problem}{voice.problem}
        {:else if phase === 'preparing'}{voice.view.detail || 'Starting the microphone…'}
        {:else if phase === 'listening'}Listening <span class="aloud">to {target}</span>{#if voice.view.engine !== 'local'}<span class="via">· {engineName(voice.view.engine)}</span>{/if}
        {:else if phase === 'transcribing'}Typing the last words…
        {:else if error}{voice.view.message}
        {:else if voice.plain}Typed into {target}. Press Enter there to send.
        {:else}{voice.notice}{/if}
      </p>
      {#if phase === 'listening'}<span class="time">{Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, '0')}</span>{/if}
      <div class="actions">
        {#if phase === 'listening'}
          <button type="button" class="button" aria-label="Stop and insert" title={shortcut ? `Stop and type the last words (${shortcut}). You press Enter to send.` : 'Stop and type the last words. You press Enter to send.'} disabled={voice.acting} onmousedown={event => event.preventDefault()} onclick={() => void voice.finish()}>Stop</button>
        {/if}
        {#if error}
          <button type="button" class="button quiet" onclick={() => { voice.problem = ''; voice.cancel(); office.openVoice() }}>Voice setup</button>
          {#if again && voice.ready && voice.settings.enabled}<button type="button" class="button" disabled={voice.view.busy || voice.acting} onclick={() => void voice.toggle(again.id)}>Try again</button>{/if}
        {/if}
        <button type="button" class="close" aria-label={voice.active ? 'Cancel voice input' : 'Dismiss voice notice'} title={voice.active ? 'Cancel: stop listening; words not yet typed are dropped' : 'Dismiss'} onmousedown={event => event.preventDefault()} onclick={close}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
        </button>
      </div>
    </div>
    {#if !docked && into && !error}
      <!-- Not in front: who the words are going to, and the way back to their terminal. -->
      <button type="button" class="where" title="Show their terminal" onclick={() => office.show(into.id)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h9.5M9 4.5 12.500 8 9 11.500" /></svg><span>{into.title}</span>
      </button>
    {/if}
    {#if voice.view.partial && voice.active}
      <!-- The phrase being spoken; phrases already finished are in the terminal. -->
      <p class="words">{voice.view.partial}</p>
    {:else if phase === 'listening' && voice.view.detail}
      <p class="hint">{voice.view.detail} Start speaking; nothing is lost.</p>
    {:else if silent}
      <p class="hint">No sound from {voice.view.microphone || 'the microphone'}. <button type="button" class="link" onclick={() => voice.picking = true}>Change microphone</button></p>
    {:else if phase === 'listening' && isPhrased(voice.view.engine)}
      <p class="hint">Typed a moment after each pause.</p>
    {/if}
  </div>
{/if}

<style>
  .voice-status {
    --left: 0px;
    --right: 0px;
    position: absolute;
    top: var(--s-2);
    /* Over the middle of the work that nothing covers. */
    right: var(--right);
    left: var(--left);
    z-index: 30;
    display: grid;
    gap: 2px;
    width: min(420px, calc(100% - var(--left) - var(--right) - 2 * var(--s-3)));
    margin-inline: auto;
    padding: 5px 5px 5px var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    color: var(--ink);
    font-size: var(--t-sm);
    line-height: 1.4;
    animation: arrive var(--quick) var(--ease) both;
  }
  @keyframes arrive {
    from { transform: translateY(-4px); opacity: 0; }
  }
  :global(.terminals) .voice-status { --left: min(var(--floor), 100% - 420px); }
  :global(.terminals.paneled) .voice-status { --right: var(--panel-width); }
  /* In the terminal it types into: its top right corner, as the find bar is. */
  .voice-status.docked {
    --left: auto;
    --right: var(--s-3);
    z-index: 5;
    width: min(340px, calc(100% - 2 * var(--s-3)));
    margin-inline: 0;
  }
  .voice-status.below { top: 52px; }
  .listening { border-color: var(--work); }
  .error { border-color: var(--trouble-ink); background: var(--trouble-wash); }
  .head { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-1) var(--s-2); min-height: 28px; }
  p { overflow-wrap: anywhere; }
  .state { display: flex; flex: 1 1 6em; min-width: 0; align-items: center; gap: var(--s-2); color: var(--work); font-weight: 650; }
  /* What went wrong takes the line to itself; the ways out go under it. */
  .error .state { display: block; flex-basis: 100%; padding: 3px var(--s-2) 0 0; color: var(--trouble-ink); }
  .via { color: var(--ink-2); font-weight: 400; white-space: nowrap; }
  .time { flex: none; font-family: var(--mono); font-size: var(--t-xs); color: var(--ink-2); white-space: nowrap; }
  .actions { display: flex; flex: none; align-items: center; gap: var(--s-1); margin-left: auto; }
  .actions .button { min-height: 28px; padding: 0 var(--s-3); }
  .close { display: grid; width: 28px; height: 28px; place-items: center; border: 0; border-radius: var(--r-1); background: transparent; color: var(--ink-2); }
  .close:hover { background: var(--inset); color: var(--ink); }
  .error .close:hover { background: rgba(255, 255, 255, 0.06); }
  .close svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; }
  .meter { width: 44px; height: 6px; flex: none; border-radius: 999px; background: var(--wall); overflow: hidden; }
  .meter span { display: block; height: 100%; background: var(--work); transform-origin: left; transition: transform 80ms linear; }
  .where { display: flex; align-items: center; gap: 6px; min-width: 0; margin: 0 var(--s-2) 2px -4px; padding: 2px 4px; border: 0; border-radius: var(--r-1); background: transparent; color: var(--ink-2); font: inherit; text-align: left; }
  .where:hover { background: var(--inset); color: var(--ink); }
  .where span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .where svg { width: 14px; height: 14px; flex: none; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  /* The newest words stay in sight: a long phrase is cut at its start, not its end. */
  .words { display: flex; flex-direction: column-reverse; max-height: 2.8em; padding: 0 var(--s-2) 3px 0; overflow: hidden; font-size: var(--t-md); color: var(--ink); }
  .hint { padding: 0 var(--s-2) 3px 0; color: var(--ink-2); }
  .link { padding: 0; border: 0; background: none; color: var(--ink); font: inherit; text-decoration: underline; text-underline-offset: 2px; }
  .spin { width: 11px; height: 11px; flex: none; border: 2px solid var(--wall); border-top-color: var(--work); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .voice-status, .spin { animation: none; } }
  @media (max-width: 760px) {
    :global(.terminals) .voice-status:not(.docked) { --left: 0px; --right: 0px; }
    /* A panel has the whole window: under its title, which does not scroll. */
    :global(.paneled) .voice-status:not(.docked) { top: 64px; }
  }
</style>
