<script lang="ts">
  // Seat a new agent: which program, what to do, and where. Lives in the side
  // panel, not a dialog. Starting one starts that program in a terminal here.
  import { onMount, tick, untrack } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { joinPiece, shortcutLabel } from '../lib/voice'
  import LaunchSettings from './LaunchSettings.svelte'
  import VoiceLine from './VoiceLine.svelte'
  import { WHERE_TO_GET, type LaunchOptions } from '../lib/types'

  let { cwd: initialCwd, harness: initialHarness = '', launch: initialLaunch }: { cwd: string; harness?: string; launch?: LaunchOptions } = $props()

  // The form opens with whatever was in it when it was last closed, so a stray
  // Escape costs nothing. It is emptied when the agent starts.
  const kept = office.newDraft
  let harness = $state(untrack(() => initialHarness) || kept.harness || office.lastHarness)
  // The folder is filled in with where they most likely work: the room asked from, what was
  // typed before, the terminal in use, or where the last agent started.
  let cwd = $state(untrack(() => initialCwd) || kept.cwd || office.likelyFolder)
  let task = $state(kept.task)
  let title = $state(kept.title)
  let worktree = $state(kept.worktree)
  let launches = $state<Record<string, LaunchOptions>>(structuredClone(kept.launches))
  let starting = $state(false)
  let problem = $state('')
  let folderProblem = $state(false)
  let taskField = $state<HTMLTextAreaElement>()
  let folderField = $state<HTMLInputElement>()

  /** What can be chosen: what is on this computer, then what the office can install. */
  const choices = $derived([...office.installed, ...office.harnesses.filter(h => !h.installed && h.install_line)])
  /** Programs to install are offered on asking, so the ones already here come first and the list stays short. */
  let everything = $state(false)
  const shown = $derived(choices.filter(h => h.installed || everything || h.id === harness))
  const more = $derived(choices.length - shown.length)
  /** The program chosen, or the first one on this computer until one is. */
  const kind = $derived(choices.find(h => h.id === harness) ?? office.installed[0] ?? choices[0])
  /** Not here, and not something the office installs: the user does that themselves. */
  const missing = $derived(office.harnesses.filter(h => !h.installed && !h.install_line))
  /** Whether the program takes a model and permissions when it starts. */
  const launchable = $derived(Boolean(kind?.launch && kind.launch !== 'none'))
  /** What More options is set to, said on its folded line; empty while everything is the program's own. */
  const chosen = $derived.by(() => {
    const options = kind ? (launches[kind.id] ?? {}) : {}
    const parts = launchable ? [options.model, options.effort && `${options.effort} effort`, options.permission && options.permission !== '' ? options.permission : ''] : []
    const rest = launchable && (options.profile || options.sandbox || options.approval || options.search != null || options.chrome != null || options.instructions || options.additional_dirs?.length || options.allowed_tools?.length || options.disallowed_tools?.length || Object.keys(options.plugins ?? {}).length || options.extra_args?.length)
    return [title.trim() && `“${title.trim()}”`, ...parts, rest ? 'and more' : ''].filter((part): part is string => Boolean(part))
  })
  /** Folders used before, to pick with one click. */
  const folders = $derived(office.folders.filter(f => f !== cwd.trim()).slice(0, 4))
  const baseName = (folder: string) => folder.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || folder
  const folderLabel = (folder: string) => office.folders.filter(f => baseName(f) === baseName(folder)).length > 1 ? folder.replace(/[\\/]+$/, '').split(/[\\/]/).slice(-2).join('/') : baseName(folder)

  // Opened from a room's empty desk, the form takes that room's folder; from "Start another
  // like this", that desk's folder and program too.
  $effect(() => {
    if (initialCwd) cwd = initialCwd
  })
  $effect(() => {
    if (initialHarness) harness = initialHarness
  })
  $effect(() => {
    if (initialHarness && initialLaunch) launches[initialHarness] = structuredClone($state.snapshot(initialLaunch))
  })

  $effect(() => {
    office.newDraft = { harness, task, cwd, title, worktree, launches: $state.snapshot(launches) }
  })

  /** The form has been closed. Whatever it was waiting for then must not start an agent. */
  let closed = false
  onMount(() => {
    taskField?.focus()
    return () => {
      closed = true
    }
  })

  // ── saying the task instead of typing it ──────────────────────────────
  /** Voice is on and its engine is there to use; until then the microphone opens its setup. */
  const canTalk = $derived(voice.settings.enabled && voice.ready)
  const listening = $derived(voice.view.phase === 'listening')
  /** The text before the caret when the last spoken piece went in. The next piece carries on
   *  from it only while the caret is still there: moved, it starts afresh where it was put. */
  let spokenTo: string | null = null

  /** A settled piece of a recording goes in at the caret, as typing it would. */
  function take(text: string, space: boolean) {
    const field = taskField
    if (!field) return
    const start = field.selectionStart
    const end = field.selectionEnd
    const before = field.value.slice(0, start)
    const last = end === field.value.length
    const piece = joinPiece(before, field.value.slice(end), text, space, before === spokenTo)
    // Typed in, so Undo takes it back. A field the keyboard is not in is changed directly.
    if (document.activeElement !== field || !document.execCommand('insertText', false, piece.text)) {
      field.setRangeText(piece.text, start, end, 'end')
      field.dispatchEvent(new Event('input', { bubbles: true }))
      if (last) field.scrollTop = field.scrollHeight
    }
    const caret = start + piece.text.length - piece.back
    field.setSelectionRange(caret, caret)
    spokenTo = field.value.slice(0, caret)
  }

  // The task box takes dictation for as long as it is there: closing the form, or choosing
  // a program that takes no task, ends a recording for it.
  $effect(() => {
    const field = taskField
    if (!field) return
    return untrack(() =>
      voice.offer({
        begin: () => {
          spokenTo = null
          if (document.activeElement !== field) field.focus()
        },
        take
      })
    )
  })

  function talk() {
    if (!canTalk) office.openVoice()
    else if (listening) void voice.finish()
    else void voice.toggle('')
  }

  async function browse() {
    const picked = await bridge.pickFolder().catch(() => null)
    if (picked) {
      cwd = picked
      folderProblem = false
    }
  }

  async function start() {
    if (starting) return
    // Still dictating: the recording is finished first, so its last words are in the task.
    if (voice.form) {
      starting = true
      const whole = await voice.settle()
      starting = false
      // Cancelled while it finished, the form closed, or the recording ended in trouble:
      // nobody is started behind the back of whoever did that, or without the words.
      if (closed || !whole) return
    }
    problem = ''
    folderProblem = false
    if (!kind) {
      problem = 'None of the programs an agent can be was found on this computer.'
      return
    }
    if (!cwd.trim()) {
      folderProblem = true
      await tick()
      folderField?.focus()
      folderField?.scrollIntoView({ block: 'center' })
      return
    }
    starting = true
    const spec = {
      harness: kind.id,
      cwd: cwd.trim(),
      prompt: kind.takes_task ? task.trim() : '',
      title: title.trim(),
      worktree: kind.worktree && worktree,
      launch: kind.launch && kind.launch !== 'none' ? $state.snapshot(launches[kind.id] ?? {}) : {}
    }
    try {
      // Not here yet: installed first, in a terminal of its own, and then started.
      if (kind.installed) await office.newAgent(spec)
      else {
        await office.install(kind.id, spec)
        office.clearNewDraft()
      }
    } catch (error) {
      problem = typeof error === 'string' ? error : kind.installed ? 'The agent could not be started.' : `${kind.name} could not be installed.`
    } finally {
      starting = false
    }
  }
</script>

<aside class="panel" aria-label="New agent">
  <header>
    <h2>New agent</h2>
    <button type="button" class="close" aria-label="Close" onclick={() => office.closePanel()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
    </button>
  </header>

  <!-- Ctrl+Enter starts the agent from anywhere in the form, the task box included. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    id="new-agent"
    novalidate
    onsubmit={event => {
      event.preventDefault()
      void start()
    }}
    onkeydown={event => {
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && !starting) {
        event.preventDefault()
        // Commit native change fields, just as clicking the Start button does.
        // Multiline folder/tool rules stay unnormalized while being edited.
        if (event.target instanceof HTMLElement) event.target.blur()
        void start()
      }
    }}
  >
    <div>
      <span class="label" id="new-who">Who should take it?</span>
      {#if choices.length > 0}
        <div class="pick" role="radiogroup" aria-labelledby="new-who">
          {#each shown as h (h.id)}
            <label class:on={kind?.id === h.id} class:absent={!h.installed}>
              <input type="radio" name="harness" value={h.id} checked={kind?.id === h.id} onchange={() => (harness = h.id)} />
              {h.name}
              {#if !h.installed}<span class="get">get</span>{/if}
            </label>
          {/each}
          {#if more > 0}
            <button type="button" class="more" onclick={() => (everything = true)}>+{more} to install</button>
          {/if}
        </div>
        {#if kind && !kind.installed && kind.install_needs}
          <!-- What installs it is missing: said before the button is pressed, with where to get it. -->
          <p class="hint">
            {kind.name} is installed with npm, which comes with {kind.install_needs}, and {kind.install_needs} was not
            found on this computer. Install it first. The office finds it when you come back to this window: there is
            nothing to restart.
          </p>
          {#if WHERE_TO_GET[kind.install_needs]}
            <button type="button" class="button quiet get-it" onclick={() => bridge.openPage(WHERE_TO_GET[kind.install_needs]!)}>Get {kind.install_needs}</button>
          {/if}
        {:else if kind && !kind.installed}
          <p class="hint">
            {kind.name} is not on this computer yet. It is installed in a terminal here, where you can watch it, and then
            the agent starts:
          </p>
          <code class="line">{kind.install_line}</code>
        {:else if kind?.signed_in === false}
          <p class="hint">Nobody is signed in to {kind.name}. It asks you to sign in when it starts.</p>
        {/if}
        {#if everything && missing.length > 0}
          <p class="hint">Also known, but installed their own way: {missing.map(h => h.name).join(', ')}. Once one is, it is offered here.</p>
        {/if}
      {:else}
        <p class="todo">
          <strong>None was found on this computer.</strong> Install Claude Code, Codex or another one, check that it runs in
          a terminal, and it will be offered here.
        </p>
      {/if}
    </div>

    {#if kind?.takes_task}
      <div>
        <div class="label said">
          <label for="new-task">What should they do? <span class="optional">optional</span></label>
          <!-- It does not take the keyboard: the caret stays in the task, where the words go. -->
          <button
            type="button"
            class="talk"
            class:listening
            aria-label={!canTalk ? 'Set up voice input' : listening ? 'Stop dictating' : 'Dictate the task'}
            title={!canTalk ? 'Say the task instead of typing it: set up voice input' : listening ? 'Stop, and add the last words' : voice.settings.shortcut === 'none' ? 'Say the task instead of typing it' : `Say the task instead of typing it: click, or hold ${shortcutLabel(voice.settings.shortcut)} while you speak`}
            disabled={canTalk && (voice.changing || voice.acting || (!listening && (voice.active || voice.view.busy)))}
            onmousedown={event => event.preventDefault()}
            onclick={talk}
          >
            {#if listening}
              <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="5" y="5" width="10" height="10" rx="1" /></svg>
            {:else}
              <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="2" width="6" height="10" rx="3" /><path d="M4 9v1a6 6 0 0 0 12 0V9M10 16v2M7 18h6" /></svg>
            {/if}
          </button>
        </div>
        <textarea
          id="new-task"
          class="field"
          rows="4"
          placeholder="Fix the checkout total when a coupon is applied twice, and add a test for it."
          bind:value={task}
          bind:this={taskField}
        ></textarea>
        <VoiceLine back={() => taskField?.focus()} />
      </div>
    {:else if kind}
      <p class="heads-up">{kind.name} takes its task in its own terminal. They start in the folder and wait for you there.</p>
    {/if}

    <div>
      <div class="label"><label for="new-cwd">Folder</label> <span class="optional" aria-hidden="true">required</span></div>
      <div class="pair">
        <input id="new-cwd" class="field" placeholder="The project they work in" required aria-invalid={folderProblem || undefined} aria-describedby={folderProblem ? 'new-cwd-error' : undefined} bind:value={cwd} bind:this={folderField} oninput={() => (folderProblem = false)} />
        <button type="button" class="button quiet" onclick={browse}>Browse</button>
      </div>
      {#if folderProblem}<p id="new-cwd-error" class="problem" role="alert">Choose the folder the agent should work in.</p>{/if}
      {#if folders.length > 0}
        <div class="recent" role="group" aria-label="Folders used before">
          {#each folders as folder (folder)}
            <button type="button" class="chip" title={folder} aria-label="Use {folder}" onclick={() => { cwd = folder; folderProblem = false }}>{folderLabel(folder)}</button>
          {/each}
        </div>
      {/if}
      {#if kind?.worktree}
        <label class="check" title="{kind.name} makes a git worktree on a new branch, so several agents can change one project without colliding.">
          <input type="checkbox" bind:checked={worktree} />
          Work on a separate copy <span class="optional">git worktree</span>
        </label>
      {/if}
    </div>

    <!-- Everything with a good default, folded into one line that says what it is set to. -->
    <details class="options" bind:open={office.newOptionsOpen}>
      <summary>
        <span class="summary-title">More options</span>
        <span class="summary-now" class:set={chosen.length > 0}>{chosen.length > 0 ? chosen.join(' · ') : launchable ? 'name, model, permissions' : 'name'}</span>
      </summary>
      <div class="option-fields">
        <div>
          <label class="label" for="new-title">Name <span class="optional">optional</span></label>
          <input id="new-title" class="field" placeholder="Taken from the task if you leave it empty" bind:value={title} />
        </div>
        {#if kind && launchable}
          {#key kind.id}<LaunchSettings {kind} {cwd} bind:options={() => launches[kind.id] ?? {}, value => { launches[kind.id] = value }} />{/key}
        {/if}
      </div>
    </details>
  </form>
  <footer>
    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    <div class="end">
      <button type="submit" form="new-agent" class="button" disabled={starting || !kind}>
        {starting ? 'Starting…' : kind && !kind.installed ? `Install ${kind.name} and start` : 'Start agent'}
      </button>
      <button type="button" class="button quiet" onclick={() => office.closePanel()}>Cancel</button>
      <kbd class="keys">ctrl ↵</kbd>
    </div>
  </footer>
</aside>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    width: var(--panel-width);
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--panel);
    animation: arrive var(--settle) var(--ease) both;
  }
  @keyframes arrive {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s-4) var(--s-4) var(--s-3);
    border-bottom: 1px solid var(--line);
  }
  h2 {
    font-size: var(--t-xl);
    font-weight: 750;
    line-height: 1.2;
  }
  .close {
    display: grid;
    width: 30px;
    height: 30px;
    margin-right: -6px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .close:hover {
    background: var(--inset);
    color: var(--ink);
  }
  .close svg {
    width: 16px;
    height: 16px;
  }
  .close path {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  form {
    display: grid;
    align-content: start;
    gap: var(--s-4);
    padding: var(--s-4);
    min-height: 0;
    scroll-padding-block: var(--s-4);
    overflow-y: auto;
  }
  .pair {
    display: flex;
    gap: var(--s-2);
  }
  /* One choice of a few, held in a single control. More than fit on a line wrap under. */
  .pick {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    padding: 3px;
    border: 1px solid var(--field-line);
    border-radius: var(--r-1);
    background: var(--inset);
  }
  .pick label {
    display: grid;
    flex: 1 1 auto;
    min-height: 28px;
    padding: 0 var(--s-3);
    place-items: center;
    border-radius: 4px;
    font-size: var(--t-sm);
    font-weight: 650;
    color: var(--ink-2);
    white-space: nowrap;
    cursor: pointer;
  }
  .pick label:hover {
    color: var(--ink);
  }
  .pick label.on {
    background: var(--button);
    color: var(--button-ink);
  }
  .pick label:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .pick input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .pick label {
    grid-auto-flow: column;
    gap: 6px;
  }
  .pick label.absent:not(.on) {
    color: var(--ink-3);
  }
  .get {
    padding: 0 5px;
    border: 1px solid currentColor;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-weight: 500;
  }
  /* Dimmed only on the chosen option's light fill. On the field it is already the
     quietest ink, and dimmed as well it would be under 4.5:1. */
  .pick label.on .get {
    opacity: 0.8;
  }
  .line {
    display: block;
    margin-top: var(--s-2);
    padding: 6px 10px;
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-1);
    background: var(--term);
    font-size: var(--t-sm);
    white-space: nowrap;
    user-select: text;
  }
  .get-it {
    margin-top: var(--s-2);
  }
  .check {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: fit-content;
    margin-top: var(--s-3);
    font-size: var(--t-sm);
    font-weight: 650;
  }
  .check .optional {
    font-family: var(--mono);
    font-size: var(--t-xs);
  }
  .check input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--button);
  }
  .optional {
    margin-inline-start: var(--s-1);
    font-weight: 400;
    color: var(--ink-2);
  }
  /* The task's label, and the microphone that fills it in, at the two ends of one line. */
  .said {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2);
  }
  .talk {
    display: grid;
    flex: none;
    width: 28px;
    height: 28px;
    margin: -4px -4px -4px 0;
    place-items: center;
    border: 1px solid transparent;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .talk:hover:not(:disabled) {
    background: var(--inset);
    color: var(--ink);
  }
  .talk:disabled {
    opacity: 0.55;
  }
  .talk.listening {
    border-color: var(--work);
    color: var(--work);
  }
  .talk svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
  }
  /* Actions have their own row, so they never cover a field or its error. */
  footer {
    display: grid;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-4);
    border-top: 1px solid var(--line);
  }
  .end {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2);
  }
  /* The quietest ink at its full strength: a key cap's usual dimming on top of it
     would be under 4.5:1 on the panel. */
  .keys {
    margin-left: auto;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-3);
    opacity: 1;
  }
  .more {
    flex: 1 1 auto;
    min-height: 28px;
    padding: 0 var(--s-3);
    border: 1px dashed var(--wall);
    border-radius: 4px;
    background: transparent;
    color: var(--ink-3);
    font-family: var(--mono);
    font-size: var(--t-xs);
  }
  .more:hover {
    color: var(--ink);
  }

  /* More options, folded: its line says what it is set to. */
  .options {
    border-top: 1px solid var(--line);
    padding-top: var(--s-3);
  }
  .options summary {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
    min-width: 0;
    font-size: var(--t-sm);
    cursor: pointer;
  }
  .summary-title {
    flex: none;
    font-weight: 650;
  }
  .summary-now {
    min-width: 0;
    overflow: hidden;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-3);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .summary-now.set {
    color: var(--ink-2);
  }
  .options[open] .summary-now {
    display: none;
  }
  .option-fields {
    display: grid;
    gap: var(--s-4);
    padding-top: var(--s-3);
  }

  /* Folders used before: one click instead of typing a path. */
  .recent {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-1);
    margin-top: var(--s-2);
  }
  .chip {
    max-width: 100%;
    padding: 2px var(--s-3);
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: transparent;
    font-size: var(--t-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip:hover {
    background: var(--inset);
  }
  .todo {
    padding: var(--s-3);
    border-radius: var(--r-2);
    background: var(--inset);
    line-height: 1.45;
  }
  .heads-up {
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--ink-2);
  }
</style>
