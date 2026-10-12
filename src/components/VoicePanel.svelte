<script lang="ts">
  import { onMount } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { isProvider, PROVIDERS, shortcutLabel, vocabularyWords, type VoiceProvider, type VoiceSettings } from '../lib/voice'
  let first = $state<HTMLButtonElement>()
  onMount(() => { first?.focus(); void voice.refresh(); void voice.loadInputs() })
  const busy = $derived(voice.changing || voice.view.verifying || !!voice.view.downloading)
  const local = $derived(voice.settings.engine === 'local')
  const selected = $derived(voice.view.models.find(m => m.id === voice.settings.local_model))
  const other = $derived(voice.view.models.find(m => m.id !== voice.settings.local_model && m.ready))
  const provider = $derived(isProvider(voice.settings.engine) ? voice.view.providers.find(p => p.id === voice.settings.engine) : undefined)
  const missingMic = $derived(!!voice.settings.microphone && !voice.inputsLoading && !voice.inputs.devices.includes(voice.settings.microphone))
  const mb = (bytes: number) => `${(bytes / 1_000_000).toFixed(0)} MB`

  // The key being typed lives here only until it is handed to the core, and is never shown again.
  let editing = $state(false)
  let draft = $state('')
  let keyBusy = $state(false)
  let keyNote = $state<{ text: string; offer: 'session' | 'unchecked' | null; ok: boolean } | null>(null)
  let words = $state('')
  $effect(() => { void voice.settings.engine; editing = false; draft = ''; keyNote = null })

  function useCloud() {
    // A provider that already has a key, else the first.
    const ready = voice.view.providers.find(p => p.configured)
    void voice.configure({ engine: ready?.id ?? 'deepgram' })
  }
  async function save(sessionOnly: boolean, check: boolean) {
    if (!provider || keyBusy || !draft.trim()) return
    keyBusy = true
    keyNote = null
    const problem = await voice.setKey(provider.id, draft, sessionOnly, check)
    keyBusy = false
    if (problem) { keyNote = { ...problem, ok: false }; return }
    draft = ''
    editing = false
    keyNote = { text: sessionOnly ? 'Kept until Agent Moshpit closes. Add it again next time, or set up the system keychain.' : check ? `${provider.name} accepted the key.` : 'Key saved without checking. Test it once you are online.', offer: null, ok: true }
  }
  async function test(id: VoiceProvider, name: string) {
    keyBusy = true
    keyNote = null
    const problem = await voice.checkKey(id)
    keyBusy = false
    keyNote = problem ? { ...problem, offer: null, ok: false } : { text: `${name} accepted the key.`, offer: null, ok: true }
  }
  function addWords(event: SubmitEvent) {
    event.preventDefault()
    const vocabulary = vocabularyWords(words, voice.settings.vocabulary)
    words = ''
    if (vocabulary.length !== voice.settings.vocabulary.length) void voice.configure({ vocabulary })
  }
</script>

<aside class="panel" aria-label="Voice input">
  <header><h2>Voice input</h2><button type="button" class="close" aria-label="Close" bind:this={first} onclick={() => office.closePanel()}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg></button></header>
  <div class="body">
    <p>Speak instead of typing, in English. Your words appear as you talk and are typed into the terminal as they settle. Review them, then press Enter to send.</p>
    <label class="enable"><input type="checkbox" checked={voice.settings.enabled} disabled={voice.changing} onchange={e => void voice.configure({ enabled: e.currentTarget.checked })} />Enable voice input</label>
    <div class="readiness" role="status">
      {#if !voice.settings.enabled}<p>Enable voice, then choose where your speech is turned into words.</p>
      {:else if local && voice.view.unsupported}<p>{voice.view.unsupported} Choose Cloud below to use voice on this computer.</p>
      {:else if local && voice.view.verifying}<p>Checking your downloaded models…</p>
      {:else if local && !voice.ready}<p>Download {selected?.label ?? 'the speech model'} to finish setup{other ? `, or use ${other.label}, which is already downloaded` : ''}.</p>
        {#if other && !voice.view.downloading}<button type="button" class="button" disabled={busy} onclick={() => void voice.configure({ local_model: other.id })}>Use {other.label} · downloaded</button>{/if}
        {#if selected && !voice.view.downloading}<button type="button" class="button" disabled={busy} onclick={() => void voice.download(selected.id)}>Download {selected.label} · {mb(selected.bytes)}</button>{/if}
      {:else if !local && !voice.ready}<p>Add your {provider?.name ?? 'provider'} key below to finish setup.</p>
      {:else if voice.inputsLoading}<p>Finding microphones…</p>
      {:else if voice.inputsProblem}<p>Refresh microphones to check your input.</p>
      {:else if missingMic}<p>Choose an available microphone to finish setup.</p>
      {:else if !voice.settings.microphone && !voice.inputs.default}<p>Choose a microphone below, or connect one and refresh.</p>
      {:else}<p>Ready. Close setup, select a running terminal, and press Voice{voice.settings.shortcut !== 'none' ? `, or hold ${shortcutLabel(voice.settings.shortcut)} while you speak` : ''}. In New agent, the microphone beside the task takes what you say as the task.</p>{/if}
    </div>
    {#if bridge.demo}<p class="note">Demo: simulated audio, downloads and keys. No microphone or network is used.</p>{/if}

    <section aria-labelledby="voice-engine">
      <h3 id="voice-engine">Speech engine</h3>
      <label class="choice"><input type="radio" name="voice-engine" checked={local} disabled={voice.changing} onchange={() => void voice.configure({ engine: 'local' })} /><span><strong>On this computer · English</strong><small>NVIDIA Nemotron, downloaded once. Nothing you say leaves this computer.</small></span></label>
      <label class="choice"><input type="radio" name="voice-engine" checked={!local} disabled={voice.changing} onchange={useCloud} /><span><strong>Cloud · your own key</strong><small>Deepgram, OpenAI, AssemblyAI or ElevenLabs, paid by you. Your audio goes to the provider while you dictate.</small></span></label>
    </section>

    {#if local}
      <section aria-labelledby="voice-models">
        <h3 id="voice-models">Local model</h3>
        <p class="note">Downloaded from Hugging Face when you choose; no account needed. While in use it takes about 1 GB of memory, freed 5 minutes after your last recording.</p>
        {#if voice.view.unsupported}<p class="error" role="alert">{voice.view.unsupported}</p>{/if}
        {#if voice.view.verifying}<p role="status">Checking downloaded models…</p>{/if}
        {#each voice.view.models as model (model.id)}
          <div class="model">
            <label><input type="radio" name="voice-model" checked={voice.settings.local_model === model.id} disabled={busy} onchange={() => void voice.configure({ local_model: model.id })} /><span>{model.label}<small>{mb(model.bytes)} · {model.ready ? 'ready' : 'not downloaded'}{model.recommended ? ' · recommended' : ''}</small></span></label>
            {#if model.ready}<button type="button" class="button quiet" disabled={busy || voice.view.busy} aria-label="Remove {model.label}" onclick={() => void voice.remove(model.id)}>Remove</button>
            {:else}<button type="button" class="button quiet" disabled={busy} aria-label="Download {model.label}" onclick={() => void voice.download(model.id)}>Download</button>{/if}
          </div>
        {/each}
        {#if voice.view.downloading}
          {@const model = voice.view.models.find(m => m.id === voice.view.downloading)}
          <div class="download" role="status"><progress value={voice.view.received} max={model?.bytes ?? 1} aria-label="Model download"></progress><span>{(voice.view.received / 1_000_000).toFixed(1)} of {mb(model?.bytes ?? 0)} · checked before use</span><button type="button" class="button quiet" onclick={() => void voice.cancelDownload()}>Cancel download</button></div>
        {/if}
        {#if voice.view.download_error}<p class="error" role="alert">{voice.view.download_error}</p>{/if}
      </section>
    {:else if provider}
      {@const about = PROVIDERS[provider.id]}
      <section>
        <label for="voice-provider-choice">Provider</label>
        <select id="voice-provider-choice" value={provider.id} disabled={voice.changing} onchange={e => void voice.configure({ engine: e.currentTarget.value as VoiceSettings['engine'] })}>
          {#each voice.view.providers as p (p.id)}<option value={p.id}>{p.name} · {PROVIDERS[p.id].model}{p.configured ? ' · key added' : ''}</option>{/each}
        </select>
        <p class="consent">While you dictate, your microphone's audio goes to {provider.name} over an encrypted connection, and {provider.name} bills your account for it. {about.privacy}</p>
        <p class="links"><button type="button" class="link" onclick={() => bridge.openPage(about.keys)}>Get a {provider.name} key</button><button type="button" class="link" onclick={() => bridge.openPage(about.pricing)}>{provider.name} pricing</button></p>
        <div class="key">
          {#if provider.configured && !editing}
            <p><strong>Key added</strong> · ends in {provider.last4} · {provider.session_only ? 'kept for this session only' : 'in the system keychain'}</p>
            <div class="row">
              <button type="button" class="button quiet" disabled={keyBusy} onclick={() => void test(provider.id, provider.name)}>Test</button>
              <button type="button" class="button quiet" disabled={keyBusy} onclick={() => { editing = true; keyNote = null }}>Replace</button>
              <button type="button" class="button quiet" disabled={keyBusy} onclick={() => { keyNote = null; void voice.removeKey(provider.id) }}>Remove</button>
            </div>
          {:else if editing || !provider.configured}
            <form onsubmit={e => { e.preventDefault(); void save(false, true) }}>
              <label for="voice-key">{provider.name} API key</label>
              <div class="row">
                <input id="voice-key" type="password" autocomplete="off" spellcheck="false" placeholder="Paste the key" bind:value={draft} disabled={keyBusy} />
                <button type="submit" class="button" disabled={keyBusy || !draft.trim()}>{keyBusy ? 'Checking…' : 'Save'}</button>
                {#if provider.configured}<button type="button" class="button quiet" disabled={keyBusy} onclick={() => { editing = false; draft = ''; keyNote = null }}>Cancel</button>{/if}
              </div>
            </form>
            <p class="note">Checked with {provider.name}, then kept in your system keychain. It is never shown again or written to a file.</p>
          {/if}
          {#if keyNote}
            <p class={keyNote.ok ? 'note' : 'error'} role={keyNote.ok ? 'status' : 'alert'}>{keyNote.text}</p>
            {#if keyNote.offer === 'session'}<button type="button" class="button quiet" disabled={keyBusy} onclick={() => void save(true, false)}>Use the key for this session only</button>{/if}
            {#if keyNote.offer === 'unchecked'}<button type="button" class="button quiet" disabled={keyBusy} onclick={() => void save(false, false)}>Save without checking</button>{/if}
          {/if}
        </div>
      </section>
      <section aria-labelledby="voice-words-heading">
        <h3 id="voice-words-heading">Word hints</h3>
        <p class="note">Names and terms {provider.name} should expect, sent with the audio. Program names such as Claude and Codex are always included.</p>
        {#if voice.settings.vocabulary.length}
          <ul class="chips">{#each voice.settings.vocabulary as word (word)}<li>{word}<button type="button" aria-label="Remove {word}" disabled={voice.changing} onclick={() => void voice.configure({ vocabulary: voice.settings.vocabulary.filter(w => w !== word) })}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg></button></li>{/each}</ul>
        {/if}
        <form class="row" onsubmit={addWords}>
          <input aria-label="Add word hints" placeholder="Add words, separated by commas" bind:value={words} disabled={voice.changing} />
          <button type="submit" class="button quiet" disabled={voice.changing || !words.trim()}>Add</button>
        </form>
        <label class="choice"><input type="checkbox" checked={voice.settings.send_desk_names} disabled={voice.changing} onchange={e => void voice.configure({ send_desk_names: e.currentTarget.checked })} /><span>Also send the desk's name, folder and branch</span></label>
      </section>
    {/if}

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
    <section>
      <label for="voice-shortcut">Shortcut</label>
      <select id="voice-shortcut" value={voice.settings.shortcut} disabled={voice.changing} onchange={e => void voice.configure({ shortcut: e.currentTarget.value as VoiceSettings['shortcut'] })}>{#each ['space', 'altspace', 'none'] as shortcut}<option value={shortcut}>{shortcutLabel(shortcut as VoiceSettings['shortcut'])}</option>{/each}</select>
      <p class="note">Hold it while you speak and let go to finish, or tap it to talk hands-free and tap again to finish. Works inside this window while voice is enabled; change or turn it off if your program uses the same key.</p>
    </section>
    <p class="note">Recordings stay in memory and are discarded when they end. Cancelling stops the recording; words already typed stay in the terminal for you to keep or delete. Voice input leaves your clipboard alone.</p>
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
  .enable, .model label, .choice { display: flex; align-items: center; gap: var(--s-2); }
  .choice { align-items: flex-start; font-weight: 400; }
  .choice strong { font-weight: 650; }
  .choice input { margin-top: 3px; }
  .enable { font-weight: 650; }
  input[type='checkbox'], input[type='radio'] { accent-color: var(--work); flex: none; }
  section { display: grid; gap: var(--s-2); margin-top: var(--s-3); padding-top: var(--s-3); border-top: 1px solid var(--line); }
  h3, section > label:not(.choice), .key label { font-size: var(--t-md); font-weight: 650; }
  .readiness { display: grid; gap: var(--s-2); padding: var(--s-3); background: var(--inset); border-radius: var(--r-1); }
  .readiness button { justify-self: start; white-space: normal; text-align: left; }
  .section-heading { display: flex; align-items: center; justify-content: space-between; gap: var(--s-2); }
  .section-heading label { font-weight: 650; }
  .section-heading button { min-height: 32px; padding-inline: var(--s-2); font-size: var(--t-sm); }
  .model { display: flex; justify-content: space-between; align-items: center; gap: var(--s-2); padding: var(--s-2) 0; }
  .model label { min-width: 0; }
  small { display: block; margin-top: 2px; }
  .model button { padding: 0 8px; flex: none; }
  select, input:not([type='checkbox'], [type='radio']) { width: 100%; min-width: 0; padding: 7px var(--s-2); border: 1px solid var(--field-line); border-radius: var(--r-1); background: var(--inset); color: var(--ink); font: inherit; }
  .consent { padding: var(--s-2) var(--s-3); border-left: 2px solid var(--work); background: var(--inset); border-radius: 0 var(--r-1) var(--r-1) 0; font-size: var(--t-sm); }
  .links { display: flex; flex-wrap: wrap; gap: var(--s-3); font-size: var(--t-sm); }
  .link { padding: 0; border: 0; background: none; color: var(--work); font: inherit; text-decoration: underline; text-underline-offset: 2px; cursor: pointer; }
  .key { display: grid; gap: var(--s-2); }
  .key form { display: grid; gap: var(--s-2); }
  .key > button { justify-self: start; }
  .row { display: flex; gap: var(--s-2); align-items: center; }
  .row .button { flex: none; padding: 0 var(--s-3); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--s-1); padding: 0; margin: 0; list-style: none; }
  .chips li { display: inline-flex; align-items: center; gap: 2px; padding: 2px 2px 2px var(--s-2); border: 1px solid var(--line); border-radius: var(--pill); background: var(--inset); font-size: var(--t-sm); max-width: 100%; overflow-wrap: anywhere; }
  .chips button { display: grid; width: 22px; height: 22px; flex: none; place-items: center; border: 0; border-radius: 50%; background: transparent; color: var(--ink-2); }
  .chips button:hover { background: var(--wall); color: var(--ink); }
  .chips svg { width: 12px; height: 12px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }
  .download { display: grid; gap: var(--s-2); font-size: var(--t-sm); }
  progress { width: 100%; accent-color: var(--work); }
  .error { color: var(--trouble-ink); overflow-wrap: anywhere; }
</style>
