<script lang="ts">
  // The open terminals, sharing the room: one, two side by side, or a grid of any
  // number. Each is headed by its worker, and every edge between two can be dragged.
  //
  // Panes are placed by position, not nested in rows, and kept in the page in an order
  // that never changes while they are open. So a terminal is never torn down and made
  // again when the room changes shape around it, and never moved in the page either:
  // a terminal that is moved loses the keyboard.
  import { aim, land, plan as laidOut, type Aim, type Edge } from '../lib/layout'
  import { openDeskMenu } from '../lib/menus'
  import { office, type MenuItem } from '../lib/office.svelte'
  import { terms } from '../lib/terms'
  import { elapsed } from '../lib/time'
  import type { Agent, Job } from '../lib/types'
  import { voice } from '../lib/voice.svelte'
  import { STATUS_WORD, doing, place, resumeAction, resumeHint } from '../lib/words'
  import Person from './Person.svelte'
  import VoiceStatus from './VoiceStatus.svelte'
  const terminalView = import('./TerminalView.svelte')

  /** Where each pane and each edge is, as shares of the room. Panes come in reading order. */
  const plan = $derived(laidOut(office.layout))
  /** The panes in the order they are kept in the page: by desk, whatever the room looks like. */
  const kept = $derived([...plan.panes].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)))
  /** What a pane is called in the page, so the room can say what order they are read in. */
  const named = (id: string) => `pane-${id}`

  const agentOf = (id: string): Agent | undefined => office.agents.find(a => a.id === id)
  const jobOf = (id: string): Job | undefined => office.jobs.find(j => j.id === id)
  /** What a job's header says it is doing. */
  const jobWords = (job: Job) => {
    const name = job.harness_name
    if (job.kind === 'signin') return job.running ? `signing in to ${name}` : job.ok ? `${name} signed in` : `sign-in to ${name} did not finish`
    if (job.running) return `${job.kind === 'install' ? 'installing' : 'updating'} ${name}`
    return job.ok ? `${name} ${job.kind === 'install' ? 'installed' : 'updated'}` : `${job.kind === 'install' ? 'install' : 'update'} of ${name} failed`
  }
  /**
   * Whether nobody is signed in to a desk's program, as that program last said. It then
   * asks to be signed in to when it starts, so that is what its pane offers.
   */
  const signedOut = (agent: Agent) => office.harnesses.find(h => h.id === agent.harness)?.signed_in === false
  const percent = (share: number) => `${(share * 100).toFixed(4)}%`

  let room = $state<HTMLElement>()
  let dragging = $state('')
  /** A pane being dragged by its header, and where it would go, and the room it would have, if it were let go now. */
  let moving = $state('')
  let landing = $state.raw<{ to: Aim; left: number; top: number; width: number; height: number } | null>(null)
  let press: { x: number; y: number; id: string } | null = null
  /** What the find bar is looking for, and whether it found it. */
  let needle = $state('')
  let missing = $state(false)

  /** How far along its own split the pointer is: what an edge held there would be moved to. */
  function where(event: PointerEvent, edge: Edge): number {
    const box = room!.getBoundingClientRect()
    const across = edge.kind === 'column' ? (event.clientX - box.left) / box.width : (event.clientY - box.top) / box.height
    return edge.span > 0 ? (across - edge.from) / edge.span : edge.at
  }

  /** No pane is dragged narrower or shorter than a terminal can be read in. */
  function move(edge: Edge, at: number) {
    const box = room!.getBoundingClientRect()
    office.dragEdge(edge.path, edge.index, at, { row: 160 / box.width, column: 110 / box.height })
  }

  function onEdgeDown(event: PointerEvent, edge: Edge) {
    if (event.button !== 0) return
    event.preventDefault()
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    dragging = edge.key
  }

  function onEdgeMove(event: PointerEvent, edge: Edge) {
    if (dragging === edge.key) move(edge, where(event, edge))
  }

  function onEdgeUp(edge: Edge) {
    if (dragging !== edge.key) return
    dragging = ''
    office.keepView()
  }

  /** An edge that can be moved is a control: it takes the arrow keys too. */
  function onEdgeKey(event: KeyboardEvent, edge: Edge) {
    const back = edge.kind === 'column' ? 'ArrowLeft' : 'ArrowUp'
    const on = edge.kind === 'column' ? 'ArrowRight' : 'ArrowDown'
    if (event.key !== back && event.key !== on) return
    event.preventDefault()
    move(edge, edge.at + (event.key === on ? 0.02 : -0.02))
    office.keepView()
  }

  // ── a pane dragged by its header: onto a side of another, into its place, or to an edge of the room ──

  function onHeadDown(event: PointerEvent, id: string) {
    if (event.button !== 0 || plan.panes.length < 2 || office.zoomed) return
    if ((event.target as HTMLElement).closest('button, input, .tag')) return
    press = { x: event.clientX, y: event.clientY, id }
  }

  /**
   * Where the pane held at this place in the window would go, and the room it would
   * have there. Worked out from the layout as it is, by the same rules that move it
   * when it is let go, so what is ringed is where it lands. Nothing moves until then:
   * no terminal is resized, and no program draws itself again, while a pane is held.
   */
  function aimed(x: number, y: number) {
    if (!room || !moving) return null
    const box = room.getBoundingClientRect()
    const to = aim(office.layout, moving, x - box.left, y - box.top, { width: box.width, height: box.height })
    const there = to ? laidOut(land(office.layout, moving, to)).panes.find(pane => pane.id === moving) : undefined
    return to && there ? { to, left: there.left, top: there.top, width: there.width, height: there.height } : null
  }

  function onWindowMove(event: PointerEvent) {
    if (!press) return
    if (!moving) {
      if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < 8) return
      moving = press.id
    }
    landing = aimed(event.clientX, event.clientY)
  }

  function onWindowUp(event: PointerEvent) {
    // Aimed afresh, at the room as it is now: a desk may have gone while the pane was held.
    const id = moving
    const to = aimed(event.clientX, event.clientY)?.to
    letGo()
    if (id && to) office.landPane(id, to)
  }

  /** The pane is let go and nothing moves. */
  function letGo() {
    press = null
    moving = ''
    landing = null
  }

  /** Escape puts a held pane back. Taken before the terminal sees it: to its program, Escape means stop. */
  function onWindowKey(event: KeyboardEvent) {
    if (!moving || event.key !== 'Escape') return
    event.preventDefault()
    event.stopPropagation()
    letGo()
  }

  // A pane given the whole room, or the held pane's desk going, ends the drag.
  $effect(() => {
    if (moving && (office.zoomed !== '' || !plan.panes.some(pane => pane.id === moving))) letGo()
  })

  function menuFrom(event: MouseEvent, id: string) {
    const button = (event.currentTarget as HTMLElement).getBoundingClientRect()
    openDeskMenu(id, button.right - 280, button.bottom + 6)
  }

  /** The panes waiting behind the one given the room, to switch to one, or to put them all back. */
  function behindMenu(event: MouseEvent, id: string) {
    const button = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const all = plan.panes.map(p => p.id)
    const items: MenuItem[] = all.flatMap((other, at) => {
      if (other === id) return []
      const agent = agentOf(other)
      const job = jobOf(other)
      const label = agent?.title ?? (job ? jobWords(job) : 'A terminal')
      const hint = [agent ? STATUS_WORD[agent.phase].toLowerCase() : '', at < 9 ? `Alt+${at + 1}` : ''].filter(Boolean).join(' · ')
      return [{ label, hint, run: () => office.focusNth(at + 1) }]
    })
    items.push({ label: 'Put them all back', hint: 'Ctrl+Shift+Enter', divided: true, run: () => office.toggleZoom(id) })
    office.openMenu({ x: button.left, y: button.bottom + 6, items })
  }

  // ── finding text in a terminal ──

  function find(downward: boolean) {
    const term = terms.get(office.finding)
    missing = term ? !term.find(needle, downward) : false
  }

  function closeFind() {
    const id = office.finding
    terms.get(id)?.resetFind()
    office.finding = ''
    needle = ''
    missing = false
    terms.get(id)?.focus()
  }

  let findField = $state<HTMLInputElement>()

  $effect(() => {
    if (office.finding) {
      missing = false
      terms.get(office.finding)?.resetFind()
    }
  })

  // The find bar takes the keyboard from the terminal as it opens (autofocus is refused while the terminal has it).
  $effect(() => {
    if (!findField) return
    const field = findField
    requestAnimationFrame(() => {
      field.focus()
      field.select()
    })
  })

  /** How many panes wait behind the one given the room. */
  const behind = $derived(office.zoomed ? plan.panes.length - 1 : 0)
</script>

<svelte:window onpointermove={onWindowMove} onpointerup={onWindowUp} onpointercancel={letGo} onblur={letGo} onkeydowncapture={onWindowKey} />

<section class="panes" class:dragging={dragging !== '' || moving !== ''} aria-label="Terminals" aria-owns={plan.panes.map(pane => named(pane.id)).join(' ')} bind:this={room}>
  {#each kept as pane (pane.id)}
    {@const agent = agentOf(pane.id)}
    {@const job = jobOf(pane.id)}
    {@const zoomed = office.zoomed === pane.id}
    {@const hidden = office.zoomed !== '' && !zoomed}
    {@const typing = office.focused === pane.id}
    <article
      class="pane"
      class:typing
      class:hidden
      class:dropping={office.dropTarget === pane.id}
      class:moving={moving === pane.id}
      id={named(pane.id)}
      data-pane={pane.id}
      style:left={zoomed ? '0' : percent(pane.left)}
      style:top={zoomed ? '0' : percent(pane.top)}
      style:width={zoomed ? '100%' : percent(pane.width)}
      style:height={zoomed ? '100%' : percent(pane.height)}
      aria-label={agent ? `${agent.title}, in ${agent.harness_name}` : job ? jobWords(job) : 'A terminal'}
      aria-hidden={hidden}
    >
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
      <header
        class:grabbable={plan.panes.length > 1 && !office.zoomed}
        onpointerdown={event => onHeadDown(event, pane.id)}
        ondblclick={event => {
          // The whole header, as a window's title bar: a double-click gives it the room. Renaming is F2 and the menu.
          if (!(event.target as HTMLElement).closest('button')) office.toggleZoom(pane.id)
        }}
        oncontextmenu={event => {
          if (!agent) return
          event.preventDefault()
          openDeskMenu(pane.id, event.clientX, event.clientY)
        }}
      >
        {#if agent}
          <span class="face"><Person look={agent.look} phase={agent.phase} portrait /></span>
          <div class="title">
            <h2 title={agent.title}>{agent.title}</h2>
            <!-- What the agent says it is doing, when it said; else where it is working. -->
            <p class="status" title={place(agent)}>
              <span class="chip {agent.phase}"><i class="dot {agent.phase}"></i>{STATUS_WORD[agent.phase]}</span>
              <span class="since">{elapsed(office.now - agent.since_ms)}</span>
              {#if doing(agent) && agent.running}<span class="meta said">{doing(agent)}</span>
              {:else if place(agent)}<span class="meta">{place(agent)}</span>{/if}
            </p>
          </div>
          {#if office.mixed}<span class="tag" title={agent.harness_name}>{agent.harness_tag}</span>{/if}
          {#if zoomed && behind > 0}
            <button
              type="button"
              class="behind"
              aria-haspopup="menu"
              aria-label="{behind} more {behind === 1 ? 'terminal' : 'terminals'} behind this one"
              title="The terminals behind this one: switch to one, or put them all back"
              onclick={event => behindMenu(event, pane.id)}>+{behind}</button
            >
          {/if}
          <button
            type="button"
            class="tool"
            aria-label="More for {agent.title}"
            aria-haspopup="menu"
            title="Rename, restart, stop, show the folder, remove"
            onclick={event => menuFrom(event, pane.id)}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true"><circle class="solid" cx="3" cy="8" r="1.2" /><circle class="solid" cx="8" cy="8" r="1.2" /><circle class="solid" cx="13" cy="8" r="1.2" /></svg>
          </button>
          {#if plan.panes.length > 1}
            <button
              type="button"
              class="tool"
              aria-pressed={zoomed}
              aria-label={zoomed ? 'Put the other terminals back' : 'Give this terminal the room'}
              title={zoomed ? 'Put the other terminals back (Ctrl+Shift+Enter)' : 'Give this terminal the room (Ctrl+Shift+Enter)'}
              onclick={() => office.toggleZoom(pane.id)}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                {#if zoomed}
                  <path d="M6.5 2.500v4h-4M9.500 13.500v-4h4M6.500 6.500 2.500 2.500M9.500 9.500l4 4" />
                {:else}
                  <path d="M2.500 6.500v-4h4M13.500 9.500v4h-4M2.500 2.500l4 4M13.500 13.500l-4-4" />
                {/if}
              </svg>
            </button>
          {/if}
        {:else if job}
          <span class="face job" aria-hidden="true">
            {#if job.kind === 'signin'}
              <svg viewBox="0 0 16 16"><circle cx="8" cy="5.5" r="2.5" /><path d="M3.500 13c.6-2.200 2.300-3.300 4.500-3.300s3.900 1.100 4.500 3.300" /></svg>
            {:else}
              <svg viewBox="0 0 16 16"><path d="M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10" /></svg>
            {/if}
          </span>
          <div class="title">
            <h2>{jobWords(job)}</h2>
            <p class="status">
              <span class="chip {job.running ? 'working' : job.ok ? 'done' : 'failed'}">
                <i class="dot {job.running ? 'working' : job.ok ? 'done' : 'failed'}"></i>{job.running ? 'running' : job.ok ? 'done' : 'failed'}
              </span>
              <span class="meta">{job.kind === 'signin' ? 'sign-in' : job.kind}</span>
            </p>
          </div>
        {:else}
          <div class="title"><h2>Nobody is at this desk any more</h2></div>
        {/if}
        <button
          type="button"
          class="tool"
          aria-label="Put this terminal away"
          title="Put this terminal away (Ctrl+Shift+W). Their program keeps running."
          onclick={() => office.closePane(pane.id)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.500 3.500l9 9M12.500 3.500l-9 9" /></svg>
        </button>
      </header>

      <div class="screen">
        {#await terminalView then { default: TerminalView }}
        <TerminalView id={pane.id} focused={typing} {hidden} live={agent ? agent.running : true} onfocus={() => office.focusPane(pane.id)} />
        {/await}
        {#if office.finding === pane.id}
          <form
            class="find"
            class:missing
            role="search"
            onsubmit={event => {
              event.preventDefault()
            }}
          >
            <input
              class="field"
              aria-label="Find in this terminal"
              placeholder="Find"
              bind:this={findField}
              bind:value={needle}
              oninput={() => {
                terms.get(pane.id)?.resetFind()
                if (needle) find(false)
                else missing = false
              }}
              onkeydown={event => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  find(event.shiftKey)
                } else if (event.key === 'Escape') {
                  event.preventDefault()
                  event.stopPropagation()
                  closeFind()
                }
              }}
            />
            <span class="hint">{missing ? 'not found' : 'Enter: older · Shift+Enter: newer'}</span>
            <button type="button" class="tool" aria-label="Close find" onclick={closeFind}>
              <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.500 3.500l9 9M12.500 3.500l-9 9" /></svg>
            </button>
          </form>
        {/if}
        <!-- A recording into this terminal says so here, in the terminal its words go into. -->
        {#if voice.dock === pane.id}<VoiceStatus docked below={office.finding === pane.id} />{/if}
        {#if office.dropTarget === pane.id}
          <p class="drop" aria-hidden="true">Drop to paste the paths into {agent?.title ?? 'this terminal'}</p>
        {/if}
        {#if agent && !agent.running}
          <!-- Signing out ends a program, in any terminal. Started again, it asks to be signed in to: said, not left to look like a crash. -->
          {@const out = agent.phase !== 'failed' && signedOut(agent)}
          <p class="ended" role="status">
            <span>{agent.phase === 'failed' ? agent.activity || 'Their program stopped with an error.' : out ? `${agent.harness_name} is signed out.` : 'Their program is not running.'}</span>
            {#if agent.resume_note}<span class="resume-note">{agent.resume_note}</span>{/if}
            {#if agent.resumable && (agent.resume_scope === 'folder' || agent.resume_scope === 'latest')}<span class="resume-note">{resumeHint(agent)}</span>{/if}
            <button type="button" class="button" title={out ? `Starts ${agent.harness_name} here, where it asks you to sign in. ${resumeHint(agent)}` : resumeHint(agent)} onclick={() => void office.wake(pane.id)}>{out ? 'Sign in' : resumeAction(agent)}</button>
          </p>
        {/if}
      </div>
    </article>
  {/each}

  {#if landing}
    <div class="landing" aria-hidden="true" style:left={percent(landing.left)} style:top={percent(landing.top)} style:width={percent(landing.width)} style:height={percent(landing.height)}></div>
  {/if}
  <p class="aloud" aria-live="polite">{office.aloud}</p>

  {#if office.zoomed === ''}
    {#each plan.edges as edge (edge.key)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="edge {edge.kind}"
        class:held={dragging === edge.key}
        role="separator"
        tabindex="0"
        aria-orientation={edge.kind === 'column' ? 'vertical' : 'horizontal'}
        aria-label={edge.kind === 'column' ? 'Width of the terminals either side' : 'Height of the terminals above and below'}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(edge.at * 100)}
        title="Drag to resize. Double-click to make them even."
        style:left={percent(edge.left)}
        style:top={percent(edge.top)}
        style:--length={percent(edge.length)}
        onpointerdown={event => onEdgeDown(event, edge)}
        onpointermove={event => onEdgeMove(event, edge)}
        onpointerup={() => onEdgeUp(edge)}
        onpointercancel={() => onEdgeUp(edge)}
        onlostpointercapture={() => onEdgeUp(edge)}
        ondblclick={() => office.evenOut()}
        onkeydown={event => onEdgeKey(event, edge)}
      ></div>
    {/each}
  {/if}
</section>

<style>
  .resume-note { max-width: 60ch; overflow-wrap: anywhere; color: var(--ink-2); }
  .panes {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--term);
  }

  .pane {
    position: absolute;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    /* One line parts a pane from the one to its right and the one below. */
    border-right: 1px solid var(--term-line);
    border-bottom: 1px solid var(--term-line);
    background: var(--term);
  }
  .pane.hidden {
    visibility: hidden;
  }
  /* Files over it, about to be pasted into it. */
  .pane.dropping {
    box-shadow: inset 0 0 0 2px var(--work);
  }
  /* Where a pane being dragged would land: the room it would have, ringed, over whatever is there now. */
  .landing {
    position: absolute;
    z-index: 6;
    background: rgba(130, 170, 255, 0.08);
    box-shadow: inset 0 0 0 2px var(--work);
    pointer-events: none;
    transition:
      left var(--quick) var(--ease),
      top var(--quick) var(--ease),
      width var(--quick) var(--ease),
      height var(--quick) var(--ease);
  }
  .pane.moving {
    opacity: 0.55;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-width: 0;
    padding: 7px var(--s-2) 7px 10px;
    /* The terminal the keyboard goes to wears a line of ink along its top. */
    border-top: 2px solid transparent;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
    container: pane-head / inline-size;
  }
  header.grabbable {
    cursor: grab;
  }
  .dragging header.grabbable {
    cursor: grabbing;
  }
  .typing header {
    border-top-color: var(--ink);
  }
  .face {
    flex: none;
    width: 30px;
    border-radius: var(--r-1);
    background: var(--inset);
    overflow: hidden;
  }
  .face.job {
    display: grid;
    height: 30px;
    place-items: center;
    color: var(--ink-2);
  }
  .face.job svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .title {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
    margin-left: var(--s-1);
  }
  h2 {
    overflow: hidden;
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--ink-3);
    white-space: nowrap;
  }
  .chip {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
    color: var(--ink-2);
    text-transform: lowercase;
  }
  .chip.needs_you {
    color: var(--needs);
    font-weight: 600;
  }
  .chip.failed {
    color: var(--trouble-ink);
  }
  .since {
    flex: none;
  }
  .since::before,
  .meta::before {
    content: '·';
    margin-right: 7px;
  }
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Which program this person is: a word, never a colour. */
  .tag {
    flex: none;
    max-width: 14ch;
    padding: 0 6px;
    overflow: hidden;
    border: 1px solid var(--wall);
    border-radius: 5px;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.6;
    text-transform: lowercase;
    color: var(--ink-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* How many terminals wait behind the one given the room. */
  .behind {
    flex: none;
    height: 22px;
    padding: 0 7px;
    border: 1px solid var(--wall);
    border-radius: 999px;
    background: transparent;
    color: var(--ink-2);
    font-family: var(--mono);
    font-size: 11px;
  }
  .behind:hover {
    background: var(--inset);
    color: var(--ink);
  }
  /* A narrow pane keeps who it is and how they are; where and what program give way. */
  @container pane-head (max-width: 430px) {
    .meta,
    .tag {
      display: none;
    }
  }
  @container pane-head (max-width: 260px) {
    .since {
      display: none;
    }
  }

  .tool {
    display: grid;
    flex: none;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .tool:hover,
  .tool[aria-pressed='true'] {
    background: var(--inset);
    color: var(--ink);
  }
  .tool svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .tool .solid {
    fill: currentColor;
    stroke: none;
  }

  .screen {
    position: relative;
    min-width: 0;
    min-height: 0;
  }
  /* Their program has ended: said over the foot of what it last printed. */
  .ended {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    /* Above the layers xterm stacks to draw its text and links. */
    z-index: 4;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2) var(--s-3);
    padding: var(--s-2) var(--s-3);
    border-top: 1px solid var(--line);
    background: var(--panel);
    font-size: var(--t-sm);
  }
  /* Finding text: a small bar over the top right of the terminal. */
  .find {
    position: absolute;
    top: var(--s-2);
    right: var(--s-3);
    z-index: 5;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    max-width: calc(100% - 2 * var(--s-3));
    padding: 4px 4px 4px var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--panel);
    box-shadow: var(--lift);
  }
  .find .field {
    width: 200px;
    min-width: 0;
    height: 28px;
    padding: 4px 8px;
  }
  .find .hint {
    overflow: hidden;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--ink-3);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .find.missing .hint {
    color: var(--trouble-ink);
  }
  .drop {
    position: absolute;
    inset: auto var(--s-3) var(--s-3);
    z-index: 5;
    padding: var(--s-2) var(--s-3);
    border-radius: var(--r-1);
    background: var(--panel);
    box-shadow: var(--lift);
    font-size: var(--t-sm);
    pointer-events: none;
  }

  /* An edge between two panes: a line to the eye, wider to the hand. */
  .edge {
    position: absolute;
    z-index: 3;
    touch-action: none;
  }
  .edge.column {
    width: 9px;
    height: var(--length);
    margin-left: -5px;
    cursor: col-resize;
  }
  .edge.row {
    width: var(--length);
    height: 9px;
    margin-top: -5px;
    cursor: row-resize;
  }
  .edge::after {
    content: '';
    position: absolute;
    inset: 0;
    background: transparent;
    transition: background var(--quick) var(--ease);
  }
  .edge.column::after {
    inset: 0 3px;
  }
  .edge.row::after {
    inset: 3px 0;
  }
  .edge:hover::after,
  .edge.held::after {
    background: var(--term-dim);
  }
  .edge:focus-visible {
    outline: none;
  }
  .edge:focus-visible::after {
    background: var(--term-ink);
  }
  /* While an edge or a pane is held, the terminals under the pointer must not take it. */
  .dragging .screen {
    pointer-events: none;
  }
</style>
