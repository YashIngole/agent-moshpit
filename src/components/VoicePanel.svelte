<script lang="ts">
  import { onMount } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { shortcutLabel, type VoiceSettings, type VoiceModel } from '../lib/voice'
  let first = $state<HTMLButtonElement>()
  onMount(() => { first?.focus(); void voice.refresh(); void voice.loadInputs() })
  const names: Record<VoiceModel, string> = { small: 'Small · larger, slower', base: 'Base · faster, recommended' }
  const busy = $derived(voice.changing || voice.view.verifying || !!voice.view.downloading)
  const selected = $derived(voice.view.models.find(m => m.id === voice.settings.model))
  const missingMic = $derived(!!voice.settings.microphone && !voice.inputsLoading && !voice.inputs.devices.includes(voice.settings.microphone))
</script>

<aside class="panel" aria-label="Voice input">
  <header><h2>Voice input</h2><button type="button" class="close" aria-label="Close" bind:this={first} onclick={() => office.closePanel()}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg></button></header>
  <div class="body">
    <p>Speak instead of typing. Stop to insert text into your terminal, review it, then press Enter to send.</p>
    <label class="enable"><input type="checkbox" checked={voice.settings.enabled} disabled={voice.changing} onchange={e => void voice.configure({ enabled: e.currentTarget.checked })} />Enable local voice</label>
    <div class="readiness" role="status">
      {#if !voice.settings.enabled}<p>Enable voice, then download a model to get started.</p>
      {:else if voice.view.verifying}<p>Checking your downloaded models…</p>
      {:else if !voice.ready}<p>Download {voice.settings.model === 'base' ? 'Base' : 'Small'} to finish setup.</p>
        {#if selected && !voice.view.downloading}<button type="button" class="button" disabled={busy} onclick={() => void voice.download(voice.settings.model)}>Download selected model · {(selected.bytes / 1_000_000).toFixed(0)} MB</button>{/if}
      {:else if voice.inputsLoading}<p>Finding microphones…</p>
      {:else if voice.inputsProblem}<p>Refresh microphones to check your input.</p>
      {:else if missingMic}<p>Choose an available microphone to finish setup.</p>
      {:else if !voice.settings.microphone && !voice.inputs.default}<p>Choose a microphone below, or connect one and refresh.</p>
      {:else}<p>Ready. Close setup, select a running terminal, and press Voice{voice.settings.shortcut !== 'none' ? ` or ${shortcutLabel(voice.settings.shortcut)}` : ''}.</p>{/if}
    </div>
    {#if bridge.demo}<p class="note">Demo: simulated audio and downloads. No microphone or network is used.</p>{/if}
    <section>
      <div class="section-heading"><label for="voice-microphone">Microphone</label><button type="button" class="button quiet" disabled={voice.inputsLoading} onclick={() => void voice.loadInputs()}>Refresh microphones</button></div>
      <select id="voice-microphone" value={voice.settings.microphone ?? ''} disabled={voice.changing || voice.inputsLoading} onchange={e => void voice.configure({ microphone: e.currentTarget.value || null })}>
        <option value="">System default{voice.inputs.default ? ` · ${voice.inputs.default}` : ''}</option>
        {#if missingMic}<option value={voice.settings.microphone ?? ''}>{voice.settings.microphone} · disconnected</option>{/if}
        {#each voice.inputs.devices as name}<option value={name}>{name}</option>{/each}
      </select>
      {#if voice.inputsLoading}<p class="note" role="status">Finding microphones…</p>{/if}
      {#if voice.inputsProblem}<p class="error" role="alert">{voice.inputsProblem}</p>
      {:else if missingMic}<p class="error" role="alert">This microphone is disconnected. Choose an available input above.</p>
      {:else if !voice.inputsLoading && !voice.inputs.devices.length}<p class="note">No microphone found. Connect one and refresh.</p>{/if}
      <p class="note">The microphone opens only when you start recording. The input meter should move when you speak.</p>
    </section>
    <section aria-labelledby="voice-models">
      <h3 id="voice-models">Local model</h3>
      <p class="note">Download from Hugging Face when you choose. No account or billing. Audio stays on this computer.</p>
      {#if voice.view.verifying}<p role="status">Checking downloaded models…</p>{/if}
      {#each [...voice.view.models].sort((a, b) => a.id === 'base' ? -1 : b.id === 'base' ? 1 : 0) as model (model.id)}
        <div class="model">
          <label><input type="radio" name="voice-model" checked={voice.settings.model === model.id} disabled={busy} onchange={() => void voice.configure({ model: model.id })} /><span>{names[model.id]}<small>{(model.bytes / 1_000_000).toFixed(1)} MB · {model.ready ? 'ready' : 'not downloaded'}</small></span></label>
          {#if model.ready}<button type="button" class="button quiet" disabled={busy || voice.view.busy} aria-label="Remove {model.id} model" onclick={() => void voice.remove(model.id)}>Remove</button>
          {:else}<button type="button" class="button quiet" disabled={busy} aria-label="Download {model.id} model" onclick={() => void voice.download(model.id)}>Download</button>{/if}
        </div>
      {/each}
      {#if voice.view.downloading}
        {@const model = voice.view.models.find(m => m.id === voice.view.downloading)}
        <div class="download" role="status"><progress value={voice.view.received} max={model?.bytes ?? 1} aria-label="Model download"></progress><span>{(voice.view.received / 1_000_000).toFixed(1)} MB received · checking before use</span><button type="button" class="button quiet" onclick={() => void voice.cancelDownload()}>Cancel download</button></div>
      {/if}
      {#if voice.view.download_error}<p class="error" role="alert">{voice.view.download_error}</p>{/if}
    </section>
    <section>
      <label for="voice-language">Speech language</label>
      <select id="voice-language" value={voice.settings.language} disabled={voice.changing} onchange={e => void voice.configure({ language: e.currentTarget.value as VoiceSettings['language'] })}><option value="auto">Detect automatically</option><option value="english">English</option><option value="hindi">Hindi</option></select>
      <p class="note">Choose your language if automatic detection gets it wrong. Both models support Hindi and English; mixed-language results vary.</p>
    </section>
    <section>
      <label for="voice-shortcut">Start / stop shortcut</label>
      <select id="voice-shortcut" value={voice.settings.shortcut} disabled={voice.changing} onchange={e => void voice.configure({ shortcut: e.currentTarget.value as VoiceSettings['shortcut'] })}>{#each ['space', 'altspace', 'none'] as shortcut}<option value={shortcut}>{shortcutLabel(shortcut as VoiceSettings['shortcut'])}</option>{/each}</select>
      <p class="note">Works inside this window while voice is enabled. Change or turn it off if your program uses the same key.</p>
    </section>
    <p class="note">Recordings and transcripts stay in memory and are discarded after use or cancellation. Voice input leaves your clipboard alone.</p>
    <p class="note">Closing or minimizing the window cancels recording.</p>
    {#if voice.problem}<p class="error" role="alert">{voice.problem}</p>{/if}
  </div>
</aside>

<style>
  .panel { display: grid; grid-template-rows: auto minmax(0, 1fr); width: var(--panel-width); min-width: 0; min-height: 0; border-left: 1px solid var(--line); background: var(--panel); }
  header { display: flex; align-items: center; justify-content: space-between; padding: var(--s-4) var(--s-4) var(--s-3); border-bottom: 1px solid var(--line); }
  h2 { font-size: var(--t-xl); font-weight: 650; letter-spacing: -0.01em; line-height: 1.2; }
  .close { display: grid; width: 30px; height: 30px; margin-right: -6px; place-items: center; border: 0; border-radius: var(--r-1); background: transparent; color: var(--ink-2); }
  .close:hover { background: var(--inset); color: var(--ink); }
  .close svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }
  .body { display: flex; flex-direction: column; gap: var(--s-3); padding: var(--s-4); overflow-y: auto; font-size: var(--t-md); line-height: 1.4; }
  .note, small { color: var(--ink-2); font-size: var(--t-sm); }
  .enable, .model label { display: flex; align-items: center; gap: var(--s-2); }
  .enable { font-weight: 650; }
  input { accent-color: var(--work); flex: none; }
  section { display: grid; gap: var(--s-2); margin-top: var(--s-3); padding-top: var(--s-3); border-top: 1px solid var(--line); }
  h3, section > label { font-size: var(--t-md); font-weight: 650; }
  .readiness { display: grid; gap: var(--s-2); padding: var(--s-3); background: var(--inset); border-radius: var(--r-1); }
  .readiness button { justify-self: start; white-space: normal; text-align: left; }
  .section-heading { display: flex; align-items: center; justify-content: space-between; gap: var(--s-2); }
  .section-heading label { font-weight: 650; }
  .section-heading button { min-height: 32px; padding-inline: var(--s-2); font-size: var(--t-sm); }
  .model { display: flex; justify-content: space-between; align-items: center; gap: var(--s-2); padding: var(--s-2) 0; }
  .model label { min-width: 0; }
  small { display: block; margin-top: 2px; }
  .model button { padding: 0 8px; flex: none; }
  select { width: 100%; padding: 7px var(--s-2); border: 1px solid var(--field-line); border-radius: var(--r-1); background: var(--inset); color: var(--ink); font: inherit; }
  .download { display: grid; gap: var(--s-2); font-size: var(--t-sm); }
  progress { width: 100%; accent-color: var(--work); }
  .error { color: var(--trouble-ink); overflow-wrap: anywhere; }
</style>
