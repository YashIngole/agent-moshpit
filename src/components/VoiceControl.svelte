<script lang="ts">
  // The microphone in the top bar: one press starts or stops a recording, and the small
  // arrow beside it chooses which microphone and which engine, without a trip to setup.
  import { tick, untrack } from 'svelte'
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { engineName, shortcutLabel, type VoiceEngine } from '../lib/voice'

  const agent = $derived(office.agents.find(a => a.id === office.voicePane && a.running))
  /** Somewhere for the words: a terminal in front, or a field that takes them (New agent's task). */
  const setup = $derived(!voice.settings.enabled || !voice.ready || (!voice.form && (!agent || !!office.panel)))
  const into = $derived(voice.form ? 'the task' : agent?.title)
  const listening = $derived(voice.view.phase === 'listening')
  const label = $derived(listening ? 'Stop recording' : voice.active ? 'Voice input in progress' : setup ? 'Set up voice input' : 'Start voice input')
  /** The microphone chosen has gone: it is still listed, so it can be seen and left. */
  const missing = $derived(!!voice.settings.microphone && !voice.inputsLoading && !voice.inputs.devices.includes(voice.settings.microphone))

  let root = $state<HTMLElement>()

  // Open from the arrow, or from a recording that hears nothing and offers another microphone.
  $effect(() => {
    if (!voice.picking) return
    // Asked each time it opens: a headset plugged in a moment ago is in the list.
    untrack(() => void voice.loadInputs())
    void tick().then(() => items()[0]?.focus())
  })

  function close(returnFocus = false) {
    voice.picking = false
    if (returnFocus) root?.querySelector<HTMLElement>('.caret')?.focus()
  }

  function items() {
    return [...(root?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])]
  }

  function onDocumentPointer(event: PointerEvent) {
    if (!voice.picking || !root || root.contains(event.target as Node)) return
    // The link that opened it is not a press outside it.
    if (event.target instanceof Element && event.target.closest('.voice-status .link')) return
    close()
  }

  function onKey(event: KeyboardEvent) {
    if (!voice.picking) {
      if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && (event.target as HTMLElement).closest('.caret')) {
        event.preventDefault()
        voice.picking = true
      }
      return
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      close(true)
    } else if (event.key === 'Tab') {
      close(true)
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault()
      event.stopPropagation()
      const list = items()
      const at = list.indexOf(document.activeElement as HTMLElement)
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? list.length - 1 : (at + (event.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length
      list[next]?.focus()
    }
  }

  function useMicrophone(name: string | null) {
    close(true)
    if ((voice.settings.microphone ?? null) !== name) void voice.retune({ microphone: name })
  }

  function useEngine(engine: VoiceEngine) {
    close(true)
    if (voice.settings.engine !== engine) void voice.retune({ engine })
  }
</script>

<svelte:document onpointerdown={onDocumentPointer} />

<div class="voice" class:listening bind:this={root} onkeydown={onKey} role="presentation">
  <!-- It does not take the keyboard from the terminal the words are going into. -->
  <button type="button" class="mic"
    aria-label={label}
    title={listening ? 'Stop, and type the last words' : voice.active ? 'Voice input is starting or finishing' : setup ? 'Open voice input setup' : voice.settings.shortcut === 'none' ? `Talk into ${into}` : `Talk into ${into}: click, or hold ${shortcutLabel(voice.settings.shortcut)} while you speak`}
    disabled={voice.changing || voice.acting || (!listening && (voice.active || voice.view.busy))}
    onmousedown={event => event.preventDefault()}
    onclick={() => setup && !voice.active ? office.openVoice() : listening ? void voice.finish() : void voice.toggle(agent?.id ?? '')}>
    {#if listening}
      <svg viewBox="0 0 20 20" aria-hidden="true"><rect class="stop" x="5.5" y="5.5" width="9" height="9" rx="1.5" /></svg>
    {:else}
      <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="2" width="6" height="10" rx="3" /><path d="M4 9v1a6 6 0 0 0 12 0V9M10 16v2M7 18h6" /></svg>
    {/if}
  </button>
  <button type="button" class="caret" aria-label="Microphone and speech engine" aria-haspopup="menu" aria-expanded={voice.picking} title="Choose the microphone" disabled={voice.changing} onclick={() => (voice.picking ? close() : (voice.picking = true))}>
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 3.750 5 6.750l3-3" /></svg>
  </button>
  {#if voice.picking}
    <div class="sheet" role="menu" aria-label="Voice input">
      <p class="label" id="voice-mics">Microphone</p>
      <div role="group" aria-labelledby="voice-mics">
        <button type="button" role="menuitemradio" tabindex="-1" aria-checked={!voice.settings.microphone} onclick={() => useMicrophone(null)}>
          <span class="name">System default{#if voice.inputs.default}<small>{voice.inputs.default}</small>{/if}</span><i class="chosen" aria-hidden="true"></i>
        </button>
        {#if missing}
          <button type="button" role="menuitemradio" tabindex="-1" aria-checked="true" onclick={() => close(true)}>
            <span class="name">{voice.settings.microphone}<small>disconnected</small></span><i class="chosen" aria-hidden="true"></i>
          </button>
        {/if}
        {#each voice.inputs.devices as name (name)}
          <button type="button" role="menuitemradio" tabindex="-1" aria-checked={voice.settings.microphone === name} onclick={() => useMicrophone(name)}>
            <span class="name">{name}</span><i class="chosen" aria-hidden="true"></i>
          </button>
        {/each}
      </div>
      {#if voice.inputsProblem}<p class="note error" role="alert">{voice.inputsProblem}</p>
      {:else if voice.inputsLoading && !voice.inputs.devices.length}<p class="note" role="status">Finding microphones…</p>
      {:else if !voice.inputsLoading && !voice.inputs.devices.length}<p class="note">No microphone found. Connect one, then open this again.</p>{/if}
      {#if voice.engines.length > 1}
        <hr />
        <p class="label" id="voice-engines">Speech engine</p>
        <div role="group" aria-labelledby="voice-engines">
          {#each voice.engines as engine (engine)}
            <button type="button" role="menuitemradio" tabindex="-1" aria-checked={voice.settings.engine === engine} onclick={() => useEngine(engine)}>
              <span class="name">{engine === 'local' ? 'On this computer' : engineName(engine)}</span><i class="chosen" aria-hidden="true"></i>
            </button>
          {/each}
        </div>
      {/if}
      <hr />
      <button type="button" role="menuitem" tabindex="-1" onclick={() => { close(); office.openVoice() }}><span class="name">Voice settings…</span></button>
    </div>
  {/if}
</div>

<style>
  /* One control in two parts: the same width whether it is at rest or listening, so nothing beside it moves. */
  .voice {
    position: relative;
    display: flex;
    flex: none;
    height: 32px;
    border: 1px solid var(--line);
    border-radius: var(--r-1);
  }
  .voice.listening { border-color: var(--work); }
  .mic,
  .caret {
    display: grid;
    height: 100%;
    place-items: center;
    border: 0;
    background: transparent;
    color: var(--ink-2);
  }
  .mic { width: 32px; border-radius: 6px 0 0 6px; }
  .caret { width: 18px; border-radius: 0 6px 6px 0; }
  .mic:hover:not(:disabled),
  .caret:hover:not(:disabled),
  .caret[aria-expanded='true'] { background: var(--inset); color: var(--ink); }
  .mic:disabled,
  .caret:disabled { opacity: 0.55; }
  .listening .mic { color: var(--work); }
  .mic svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  .mic .stop { fill: currentColor; stroke: none; }
  .caret svg { width: 10px; height: 10px; fill: none; stroke: currentColor; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; }

  .sheet {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 40;
    display: grid;
    gap: 2px;
    width: 268px;
    max-width: calc(100vw - 2 * var(--s-3));
    max-height: calc(100dvh - var(--bar) - var(--s-4));
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
    animation: unfold var(--quick) var(--ease) both;
    transform-origin: top right;
  }
  @keyframes unfold {
    from { transform: scale(0.96) translateY(-4px); opacity: 0; }
  }
  .sheet [role='group'] { display: grid; gap: 2px; }
  .label {
    padding: var(--s-1) var(--s-3) 2px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .sheet button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    padding: 6px var(--s-3);
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    font-size: var(--t-md);
    font-weight: 600;
    text-align: left;
  }
  .sheet button:hover { background: var(--inset); }
  .sheet button:focus-visible { background: var(--inset); outline: 2px solid var(--focus); outline-offset: -2px; }
  .name { display: grid; min-width: 0; overflow-wrap: anywhere; }
  .name small { font-size: var(--t-xs); font-weight: 400; color: var(--ink-2); }
  /* The one in use: a tick at the end of its line, in ink. */
  .chosen { flex: none; width: 14px; height: 14px; }
  [aria-checked='true'] .chosen {
    background: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'%3E%3Cpath d='M3 7.4l2.600 2.600L11 4.200' fill='none' stroke='%23e7eaf0' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E") center / 14px no-repeat;
  }
  .note { padding: 2px var(--s-3) var(--s-1); font-size: var(--t-sm); color: var(--ink-2); }
  .note.error { color: var(--trouble-ink); }
  hr { margin: var(--s-1) var(--s-1); border: 0; border-top: 1px solid var(--line); }
  @media (prefers-reduced-motion: reduce) { .sheet { animation: none; } }
  /* A window kept narrow at the side of the screen: the microphone alone; its choices are in Voice input. */
  @media (max-width: 520px) {
    .caret { display: none; }
    .mic { border-radius: 6px; }
  }
</style>
