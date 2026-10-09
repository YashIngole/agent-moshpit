<script lang="ts">
  // Seat a new agent: which program, what to do, and where. Lives in the side
  // panel, not a dialog. Starting one starts that program in a terminal here.
  import { onMount, untrack } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'

  let { cwd: initialCwd, harness: initialHarness = '' }: { cwd: string; harness?: string } = $props()

  // The form opens with whatever was in it when it was last closed, so a stray
  // Escape costs nothing. It is emptied when the agent starts.
  const kept = office.newDraft
  let harness = $state(untrack(() => initialHarness) || kept.harness || office.lastHarness)
  let cwd = $state(untrack(() => initialCwd) || kept.cwd)
  let task = $state(kept.task)
  let title = $state(kept.title)
  let worktree = $state(kept.worktree)
  let starting = $state(false)
  let problem = $state('')
  let taskField = $state<HTMLTextAreaElement>()

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
  /** Folders used before, to pick with one click. */
  const folders = $derived(office.folders.filter(f => f !== cwd.trim()).slice(0, 4))
  const baseName = (folder: string) => folder.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || folder

  // Opened from a room's empty desk, the form takes that room's folder; from "Start another
  // like this", that desk's folder and program too.
  $effect(() => {
    if (initialCwd) cwd = initialCwd
  })
  $effect(() => {
    if (initialHarness) harness = initialHarness
  })

  $effect(() => {
    office.newDraft = { harness, task, cwd, title, worktree }
  })

  onMount(() => taskField?.focus())

  async function browse() {
    const picked = await bridge.pickFolder().catch(() => null)
    if (picked) cwd = picked
  }

  async function start() {
    problem = ''
    if (!kind) {
      problem = 'None of the programs an agent can be was found on this computer.'
      return
    }
    if (!cwd.trim()) {
      problem = 'Choose the folder the agent should work in.'
      return
    }
    starting = true
    const spec = {
      harness: kind.id,
      cwd: cwd.trim(),
      prompt: kind.takes_task ? task.trim() : '',
      title: title.trim(),
      worktree: kind.worktree && worktree
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
    onsubmit={event => {
      event.preventDefault()
      void start()
    }}
    onkeydown={event => {
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && !starting) {
        event.preventDefault()
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
        {#if kind && kind.installed}
          <p class="hint">
            Your own {kind.name}{kind.version ? ` ${kind.version}` : ''}: its sign-in, settings, model and tools. Nothing is set up again here.
            {#if kind.outdated}
              <span class="newer">{kind.latest} is out.</span>
              <button type="button" class="link" onclick={() => office.update(kind.id)}>Update it</button>
            {/if}
          </p>
        {:else if kind}
          <p class="hint">
            {kind.name} is not on this computer yet. It is installed in a terminal here, where you can watch it, and then
            the agent starts:
          </p>
          <code class="line">{kind.install_line}</code>
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
        <label class="label" for="new-task">What should they do? <span class="optional">optional</span></label>
        <textarea
          id="new-task"
          class="field"
          rows="4"
          placeholder="Fix the checkout total when a coupon is applied twice, and add a test for it."
          bind:value={task}
          bind:this={taskField}
        ></textarea>
        <p class="hint">Leave it empty to start them with nothing to do, and say it in their terminal.</p>
      </div>
    {:else if kind}
      <p class="heads-up">{kind.name} takes its task in its own terminal. They start in the folder and wait for you there.</p>
    {/if}

    <div>
      <label class="label" for="new-cwd">Folder</label>
      <div class="pair">
        <input id="new-cwd" class="field" placeholder="The project they work in" bind:value={cwd} />
        <button type="button" class="button quiet" onclick={browse}>Browse</button>
      </div>
      {#if folders.length > 0}
        <div class="recent" role="group" aria-label="Folders used before">
          {#each folders as folder (folder)}
            <button type="button" class="chip" title={folder} onclick={() => (cwd = folder)}>{baseName(folder)}</button>
          {/each}
        </div>
      {/if}
    </div>

    {#if kind?.worktree}
      <div>
        <label class="check">
          <input type="checkbox" bind:checked={worktree} />
          Work on a separate copy
        </label>
        <p class="hint">
          {kind.name} makes a git worktree on a new branch, so several agents can change one project without colliding.
        </p>
      </div>
    {/if}

    <div>
      <label class="label" for="new-title">Name <span class="optional">optional</span></label>
      <input id="new-title" class="field" placeholder="Taken from the task if you leave it empty" bind:value={title} />
    </div>

    {#if missing.length > 0}
      <p class="heads-up">Also known, but installed their own way: {missing.map(h => h.name).join(', ')}. Once one is, it is offered here.</p>
    {/if}

    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}

    <div class="end">
      <button type="submit" class="button" disabled={starting || !kind}>
        {starting ? 'Starting…' : kind && !kind.installed ? `Install ${kind.name} and start` : 'Start agent'}
      </button>
      <button type="button" class="button quiet" onclick={() => office.closePanel()}>Cancel</button>
      <kbd class="keys">ctrl ↵</kbd>
    </div>
  </form>
</aside>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
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
    font-size: 10.5px;
    font-weight: 500;
  }
  /* Dimmed only on the chosen option's light fill. On the field it is already the
     quietest ink, and dimmed as well it would be under 4.5:1. */
  .pick label.on .get {
    opacity: 0.8;
  }
  .newer {
    color: var(--ink);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 3px;
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
  .check {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--t-md);
    font-weight: 650;
  }
  .check input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--button);
  }
  .optional {
    font-weight: 400;
    color: var(--ink-2);
  }
  /* Start is always in reach, however long the form is: it stays at the foot of the panel. */
  .end {
    position: sticky;
    bottom: calc(-1 * var(--s-4));
    z-index: 1;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin: 0 calc(-1 * var(--s-4)) calc(-1 * var(--s-4));
    padding: var(--s-3) var(--s-4);
    border-top: 1px solid var(--line);
    background: var(--panel);
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
