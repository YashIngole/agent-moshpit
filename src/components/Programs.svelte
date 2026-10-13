<script lang="ts">
  // The programs an agent can be: which are on this computer, at which version,
  // whether a newer one is out, who is signed in to each, and the way to install,
  // update or sign in. What runs is written beside each button, and it runs in a
  // terminal here, in plain view.
  import { onMount } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import { CLICK_KEY } from '../lib/platform'
  import { WHERE_TO_GET, type Harness, type Job } from '../lib/types'

  let first = $state<HTMLElement>()
  let problem = $state('')

  /** Here first, newest news first; then what can be installed; then the rest. */
  const rows = $derived(
    [...office.harnesses].sort((a, b) => rank(a) - rank(b))
  )
  function rank(h: Harness) {
    if (h.outdated) return 0
    if (h.installed) return 1
    return h.install_line ? 2 : 3
  }
  /** The install or update of this program that is running now, and its sign-in, if either is. */
  const fetching = (h: Harness): Job | undefined => office.jobs.find(j => j.harness === h.id && j.running && j.kind !== 'signin')
  const signing = (h: Harness): Job | undefined => office.jobs.find(j => j.harness === h.id && j.running && j.kind === 'signin')
  const groups = $derived([
    { name: 'Installed', rows: rows.filter(h => h.installed) },
    { name: 'Available', rows: rows.filter(h => !h.installed) }
  ].filter(group => group.rows.length > 0))
  /** Agents of this kind with their program running right now. */
  const busy = (h: Harness) => office.agents.filter(a => a.harness === h.id && a.running).length

  onMount(() => first?.focus())

  /** What the programs of a group need before the office can install them, and is not here. */
  const needed = (rows: Harness[]) => rows.find(h => !h.installed && h.install_needs)?.install_needs ?? ''

  async function act(h: Harness) {
    problem = ''
    try {
      if (h.installed) await office.update(h.id)
      else await office.install(h.id)
    } catch (error) {
      problem = typeof error === 'string' ? error : `That did not start.`
    }
  }

  async function signIn(h: Harness) {
    problem = ''
    try {
      await office.signIn(h.id)
    } catch (error) {
      problem = typeof error === 'string' ? error : `Signing in to ${h.name} did not start.`
    }
  }
</script>

<aside class="panel" aria-label="Agent programs">
  <header>
    <h2>Agent programs</h2>
    <button type="button" class="close" aria-label="Close" bind:this={first} onclick={() => office.closePanel()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
    </button>
  </header>

  <div class="body">
    {#each groups as group (group.name)}
    <section class="program-group" aria-label={group.name}>
    <h3>{group.name}</h3>
    {#if needed(group.rows)}
      {@const what = needed(group.rows)}
      <!-- What installs them is missing: said once, above them, with where to get it. -->
      <div class="do needs">
        <p class="note">
          The ones installed with npm need {what}, which was not found on this computer. Install it first; the office
          finds it when you come back to this window, with nothing to restart.
        </p>
        {#if WHERE_TO_GET[what]}
          <button type="button" class="button quiet" onclick={() => bridge.openPage(WHERE_TO_GET[what]!)}>get {what}</button>
        {/if}
      </div>
    {/if}
    <ul>
      {#each group.rows as h (h.id)}
        <li class:absent={!h.installed}>
          <div class="head">
            <span class="name">{h.name}</span>
            <span class="version">
              {#if h.installed}
                {h.version || 'installed'}
                {#if h.outdated}<span class="newer">→ {h.latest}</span>{/if}
              {:else}
                not installed
              {/if}
            </span>
          </div>
          {#if h.installed ? h.update_line : h.install_line}
            {@const job = fetching(h)}
            <div class="do">
              <code>{h.installed ? h.update_line : h.install_line}</code>
              {#if job}
                <!-- Running in a terminal that may have been put away: this brings it back. -->
                <button type="button" class="button quiet" title="It is running in a terminal. Show it." onclick={() => office.showJob(job.id)}>show</button>
              {:else}
                <button type="button" class="button" class:quiet={h.installed && !h.outdated} onclick={() => act(h)}>
                  {h.installed ? 'update' : 'install'}
                </button>
              {/if}
            </div>
            {#if h.installed && h.outdated && busy(h) > 0}
              <p class="note">
                {busy(h)} agent{busy(h) === 1 ? ' is' : 's are'} running it. They keep the version they started with; on
                Windows an update can fail until they stop.
              </p>
            {/if}
          {:else if !h.installed}
            <p class="note">Installed its own way. Once it is on this computer it is offered here.</p>
          {/if}
          {#if h.installed && h.login_line}
            {@const job = signing(h)}
            <!-- Who is signed in, in the program's own words; then the way to sign in, or as someone else. -->
            {#if h.account}<p class="account" class:out={h.signed_in === false}>{h.account}</p>{/if}
            <div class="do">
              <code>{h.login_line}</code>
              {#if job}
                <button type="button" class="button quiet" title="Signing in is waiting in a terminal. Show it." onclick={() => office.showJob(job.id)}>show</button>
                <button type="button" class="button quiet" title="Stop signing in. Whoever is signed in now stays signed in." onclick={() => office.stopJob(job.id)}>stop</button>
              {:else}
                <button type="button" class="button" class:quiet={h.signed_in !== false} onclick={() => signIn(h)}>
                  {h.signed_in === true ? 'switch account' : 'sign in'}
                </button>
              {/if}
            </div>
            {#if busy(h) > 0}
              <p class="note">Desks running now keep their sign-in until they are restarted.</p>
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
    </section>
    {/each}
    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    <section class="editor">
      <h3 id="editor-title">Files open in</h3>
      {#if office.editors.length > 0}
        <div class="pick" role="radiogroup" aria-labelledby="editor-title">
          {#each office.editors as e (e.id)}
            <label class:on={office.editor?.id === e.id}>
              <input type="radio" name="editor" value={e.id} checked={office.editor?.id === e.id} onchange={() => office.setEditor(e.id)} />
              {e.name}
            </label>
          {/each}
        </div>
        <p class="note">
          {CLICK_KEY} and a click on a file path in a terminal opens it in {office.editor?.name}, at its line. A desk’s menu opens its
          folder there too.
        </p>
      {:else}
        <p class="note">
          No editor was found. Cursor, VS Code, Windsurf, Antigravity and Zed are looked for; until one is here, Ctrl and a
          click on a file path shows it in its folder.
        </p>
      {/if}
    </section>
    <p class="foot">
      Newer versions are looked up with npm when the office starts and every 12 hours. Start it with
      <code>MOSHPIT_NO_UPDATE_CHECK=1</code> to stop that.
    </p>
  </div>
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
    font-weight: 650;
    letter-spacing: -0.01em;
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
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .body {
    display: grid;
    /* One column no wider than the panel, whatever long word is in it. */
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: var(--s-4);
    padding: var(--s-2) var(--s-4) var(--s-4);
    overflow-y: auto;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    /* Commands stay readable in full, within the panel. */
    grid-template-columns: minmax(0, 1fr);
    gap: var(--s-2);
    padding: var(--s-3) 0;
    border-bottom: 1px solid var(--line);
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-3);
  }
  .name {
    font-weight: 600;
  }
  .absent .name {
    color: var(--ink-2);
  }
  .version {
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-3);
    white-space: nowrap;
  }
  .newer {
    margin-left: 4px;
    color: var(--ink);
  }
  .do {
    display: flex;
    align-items: center;
    gap: var(--s-2);
  }
  .do code {
    flex: 1;
    min-width: 0;
    padding: 5px 8px;
    overflow-wrap: anywhere;
    border: 1px solid var(--line);
    border-radius: var(--r-1);
    background: var(--term);
    color: var(--ink-2);
    font-size: var(--t-xs);
    white-space: normal;
    user-select: text;
  }
  .do .button {
    flex: none;
    min-width: 74px;
  }
  .do.needs {
    align-items: start;
  }
  .do.needs .note {
    flex: 1;
    min-width: 0;
    color: var(--ink-2);
  }
  /* Who is signed in: a line of its own above the way to change it. Nobody signed in is said in full ink. */
  .account {
    overflow-wrap: anywhere;
    font-size: var(--t-sm);
    line-height: 1.4;
    color: var(--ink-2);
    user-select: text;
  }
  .account.out {
    font-weight: 600;
    color: var(--ink);
  }
  .note,
  .foot {
    font-size: var(--t-xs);
    line-height: 1.5;
    color: var(--ink-3);
  }
  .editor {
    display: grid;
    gap: var(--s-2);
  }
  .editor h3,
  .program-group h3 {
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  /* One editor of the few here, held in a single control, as the New agent form picks a program. */
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
  .foot code {
    color: var(--ink-2);
    font-size: 0.95em;
    overflow-wrap: anywhere;
  }
</style>
