// The window's state: the latest snapshot from the core, which terminals are
// open and how they share the room.
import { tick } from 'svelte'
import { SvelteSet } from 'svelte/reactivity'
import { bridge } from './bridge'
import { EMPTY, close, even, has, ids, moveColumnEdge, moveRowEdge, only, open, parse, swap, trade, type Layout } from './layout'
import type { Agent, Editor, Harness, Job, NewAgentSpec, Newer, Phase, Snapshot } from './types'

/** The side panel, when there is one. Terminals are not a panel: they are the room. */
export type Panel = { kind: 'new'; cwd: string; harness: string } | { kind: 'programs' } | { kind: 'keys' } | { kind: 'voice' } | null

/** Whether a terminal is an install or an update rather than a desk's. */
export const isJob = (id: string) => id.startsWith('job-')

export interface RoomGroup {
  repo: string
  agents: Agent[]
}

/** What has been typed into the New agent form and not yet started. */
export interface NewAgentDraft {
  harness: string
  task: string
  cwd: string
  title: string
  worktree: boolean
}

/** One choice in a small menu. */
export interface MenuItem {
  label: string
  /** A second line saying what it does. */
  hint?: string
  run: () => void
  /** Ends something: drawn in the trouble colour. */
  danger?: boolean
  /** Asked again in the menu before it runs. */
  confirm?: { text: string; yes: string }
  /** Starts a new group, with a line above it. */
  divided?: boolean
}

/** A menu open at a place in the window: a desk's, or a terminal's. */
export interface Menu {
  x: number
  y: number
  items: MenuItem[]
  /** Open on its confirmation, for an item that asks first. */
  asking?: MenuItem
  /** Open as a box to rename this desk in. */
  renaming?: string
}

/** Something said for a moment at the foot of the window, perhaps with a way to undo it. */
export interface Toast {
  text: string
  action?: { label: string; run: () => void }
}

const NO_DRAFT: NewAgentDraft = { harness: '', task: '', cwd: '', title: '', worktree: false }
const FOLDERS_KEY = 'moshpit.folders'
const MOST_FOLDERS = 6
const VIEW_KEY = 'moshpit.view'
const HINTS_KEY = 'moshpit.hints'

/** How wide the floor may be drawn beside the terminals, in pixels. */
export const FLOOR_WIDTH = { least: 200, strip: 264, usual: 420, terminals: 420 }
/** Below this the floor beside the terminals is a list of names rather than a room of desks. */
export const LIST_BELOW = 300
/** The terminals' text size, in pixels. */
export const FONT_SIZE = { least: 9, usual: 13, most: 24 }
/** How long a removed desk can be brought back. */
const UNDO_MS = 8000

/** Folders agents were started in before, newest first. Kept by the window; nothing depends on it. */
function rememberedFolders(): string[] {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(FOLDERS_KEY) ?? '[]')
    return Array.isArray(saved) ? saved.filter((f): f is string => typeof f === 'string' && f.length > 0).slice(0, MOST_FOLDERS) : []
  } catch {
    return []
  }
}

interface View {
  floor: number
  layout: Layout
  harness: string
  /** Whether the terminals were open, rather than put away, when the window last was. */
  open: boolean
  font: number
  worktree: boolean
  /** The editor file paths open in, when the user picked one. Empty: the first one there is. */
  editor: string
  /** Selecting text in a terminal copies it, as in many terminals. */
  copySelect: boolean
}

/** How the room was last laid out. Kept by the window; nothing depends on it. */
function rememberedView(): View {
  const view: View = { floor: FLOOR_WIDTH.usual, layout: EMPTY, harness: '', open: false, font: FONT_SIZE.usual, worktree: false, editor: '', copySelect: false }
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(VIEW_KEY) ?? '{}')
    if (saved && typeof saved === 'object') {
      const { floor, layout, harness, open, font, worktree, editor, copySelect } = saved as Record<string, unknown>
      if (typeof floor === 'number' && Number.isFinite(floor)) view.floor = Math.max(FLOOR_WIDTH.least, Math.round(floor))
      if (typeof harness === 'string') view.harness = harness
      if (typeof editor === 'string') view.editor = editor
      if (typeof font === 'number' && Number.isFinite(font)) view.font = Math.min(FONT_SIZE.most, Math.max(FONT_SIZE.least, Math.round(font)))
      view.open = open === true
      view.worktree = worktree === true
      view.copySelect = copySelect === true
      view.layout = parse(layout)
    }
  } catch {
    // Nothing kept, or nothing readable: the usual view.
  }
  return view
}

/** Hints already given once, which are not given again. */
function givenHints(): Set<string> {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(HINTS_KEY) ?? '[]')
    return new Set(Array.isArray(saved) ? saved.filter((h): h is string => typeof h === 'string') : [])
  } catch {
    return new Set()
  }
}

const REMEMBERED = rememberedView()

class Office {
  snapshot = $state<Snapshot | null>(null)
  /** The current time, advanced only as often as something on screen needs it. */
  now = $state(Date.now())
  panel = $state<Panel>(null)
  /** The desk that holds the floor's one tab stop; arrow keys move it (see Floor.svelte). */
  homeDesk = $state('')

  /** The terminals on screen, and how they share the room. */
  layout = $state<Layout>(EMPTY)
  /** The terminals that were on screen before the user went back to the floor. */
  stowed = $state<Layout>(REMEMBERED.layout)
  /** The pane the keyboard goes to. */
  focused = $state('')
  /** A pane given the whole room for a while. The others wait behind it. */
  zoomed = $state('')
  /** How wide the floor is while terminals are open. */
  floorWidth = $state(REMEMBERED.floor)
  /** The terminals' text size. */
  fontSize = $state(REMEMBERED.font)
  /** Selecting text in a terminal copies it. */
  copyOnSelect = $state(REMEMBERED.copySelect)
  /** Closing the window quits, rather than leaving the office in the tray. Kept by the core. */
  closeQuits = $state(false)
  /** A newer version of the office that is out, and whether it is being fetched right now. */
  newer = $state<Newer | null>(null)
  updating = $state(false)
  #problem = $state('')
  /** Something that could not be done, said once. */
  get problem() {
    return this.#problem
  }
  set problem(text: string) {
    this.#problem = text
    this.problemFile = null
  }
  /** The file the problem is in, to open and put right, when there is one. */
  problemFile = $state<{ path: string; line: number | null } | null>(null)
  /** The program the last agent was started as: the form opens on it next time. */
  lastHarness = $state(REMEMBERED.harness)
  /** The small menu that is open, if one is. */
  menu = $state<Menu | null>(null)
  /** What is being said at the foot of the window. */
  toast = $state<Toast | null>(null)
  /** The pane files are being dragged over. */
  dropTarget = $state('')
  /** The pane whose find bar is open. */
  finding = $state('')
  /** Desks removed a moment ago, which can still be brought back. */
  leaving = new SvelteSet<string>()
  /** The editors on this computer that file paths can be opened in. */
  editors = $state<Editor[]>([])
  /** The editor the user picked, if they did. */
  editorChoice = $state(REMEMBERED.editor)
  /** The editor file paths open in: the one picked, else the first there is. */
  editor = $derived(this.editors.find(e => e.id === this.editorChoice) ?? this.editors[0])

  // Closing a panel must not throw away what was being typed in it: a stray Escape
  // is easy. Half-written work is kept here until it is started or the window closes.
  newDraft: NewAgentDraft = { ...NO_DRAFT, worktree: REMEMBERED.worktree }

  clearNewDraft() {
    // Which program, and whether to work on a copy, are kept for the next agent: people start several alike.
    this.newDraft = { ...NO_DRAFT, harness: this.newDraft.harness, worktree: this.newDraft.worktree }
  }

  /** An agent was started in this folder: offer it first next time. */
  rememberFolder(cwd: string) {
    const folder = cwd.trim()
    if (!folder) return
    this.#remembered = [folder, ...this.#remembered.filter(f => f !== folder)].slice(0, MOST_FOLDERS)
    try {
      localStorage.setItem(FOLDERS_KEY, JSON.stringify(this.#remembered))
    } catch {
      // No storage (a private window): the list just lasts until the window closes.
    }
  }

  /** Every desk the core knows, removed ones that can still be brought back included. */
  allAgents = $derived(this.snapshot?.agents ?? [])
  agents = $derived(this.allAgents.filter(a => !this.leaving.has(a.id)))
  harnesses = $derived(this.snapshot?.harnesses ?? [])
  /** The programs that are on this computer, in the table's order. */
  installed = $derived(this.harnesses.filter((h: Harness) => h.installed))
  jobs = $derived(this.snapshot?.jobs ?? [])
  /** Programs on this computer with a newer version out. */
  outdated = $derived(this.harnesses.filter((h: Harness) => h.outdated))
  #remembered = $state(rememberedFolders())
  /** Folders to offer in the New agent form: where agents are working now, then where they have before. */
  folders = $derived.by((): string[] => {
    const seen = new Set<string>()
    const all = [...this.agents.map(a => a.cwd), ...this.#remembered]
    return all.filter(f => f && !seen.has(f) && seen.add(f)).slice(0, MOST_FOLDERS)
  })
  /** Agents with a hand up, longest wait first. */
  waiting = $derived(this.agents.filter(a => a.phase === 'needs_you').sort((a, b) => a.since_ms - b.since_ms))
  /** One room per project, in the order the core sends them. */
  rooms = $derived.by((): RoomGroup[] => {
    const groups: RoomGroup[] = []
    for (const agent of this.agents) {
      const repo = agent.repo || 'No folder'
      const group = groups.find(g => g.repo === repo)
      if (group) group.agents.push(agent)
      else groups.push({ repo, agents: [agent] })
    }
    return groups
  })
  /** Whether any terminal is on screen. */
  terminals = $derived(this.layout.rows.length > 0)
  /** The desks whose terminal is on screen. */
  open = $derived(new Set(ids(this.layout)))
  /** Whether the floor beside the terminals is narrow enough to be a list of names. */
  listed = $derived(this.terminals && this.floorWidth < LIST_BELOW)

  #clock: ReturnType<typeof setInterval> | undefined
  #clockMs = 0
  /** Who was in the last snapshot. Null until the first one, so nobody "arrives" when the window opens. */
  #known: Set<string> | null = null
  #arrivals = new Set<string>()
  /** What had focus before a panel opened, to hand it back on close. */
  #returnTo: HTMLElement | null = null
  #hints = givenHints()
  #toastTimer: ReturnType<typeof setTimeout> | undefined
  /** Removals waiting out their chance to be undone, by desk. */
  #removals = new Map<string, ReturnType<typeof setTimeout>>()
  /** Where the last look for someone in a given state got to, so the next look moves on. */
  #lastFound = ''

  start(): () => void {
    void bridge.snapshot().then(s => this.#take(s))
    this.findEditors()
    void bridge.settings().then(
      s => (this.closeQuits = s.close_quits),
      () => {}
    )
    void bridge.newer().then(
      newer => (this.newer = newer),
      () => {}
    )
    void bridge.startupProblems().then(
      problems => {
        if (problems.length === 0) return
        this.problem = problems.map(p => p.text).join(' ')
        const file = problems.find(p => p.file)
        if (file) this.problemFile = { path: file.file, line: file.line }
      },
      () => {}
    )
    // A click on a notification: that desk's terminal, beside whatever is open.
    const takeOpening = () =>
      void bridge.takeOpening().then(
        id => {
          if (id) this.#openWhenHere(id)
        },
        () => {}
      )
    takeOpening()
    const stops = [bridge.onSnapshot(s => this.#take(s)), bridge.onNewAgent(() => this.openNew()), bridge.onOpenDesk(takeOpening), bridge.onNewer(newer => (this.newer = newer))]
    const onVisibility = () => {
      this.#syncClock()
      this.#report()
    }
    // A removal still waiting to be undone happens now: the window is going.
    const onLeave = () => this.#removeNow()
    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('pagehide', onLeave)
    this.#syncClock()
    return () => {
      for (const stop of stops) stop()
      document.removeEventListener('visibilitychange', onVisibility)
      window.removeEventListener('pagehide', onLeave)
      clearInterval(this.#clock)
      clearTimeout(this.#toastTimer)
      this.#removeNow()
    }
  }

  #take(snapshot: Snapshot) {
    const first = this.#known === null
    if (this.#known) {
      for (const agent of snapshot.agents) if (!this.#known.has(agent.id)) this.#arrivals.add(agent.id)
    }
    this.#known = new Set(snapshot.agents.map(a => a.id))
    this.snapshot = snapshot
    this.now = Date.now()
    // A desk that was taken away takes its pane with it; so does a job that was put away.
    const jobs = new Set(snapshot.jobs.map(j => j.id))
    const alive = (id: string) => (this.#known!.has(id) && !this.leaving.has(id)) || jobs.has(id)
    const layout = only(this.layout, alive)
    if (layout !== this.layout) this.#lay(layout)
    this.stowed = only(this.stowed, alive)
    // The window was closed with terminals open and their programs kept running: they come back as they were.
    if (first && REMEMBERED.open && this.layout.rows.length === 0 && ids(this.stowed).some(id => snapshot.agents.find(a => a.id === id)?.running)) {
      this.focused = ids(this.stowed)[0] ?? ''
      this.#lay(this.stowed)
    }
    this.#syncClock()
    this.#afterInstalls(snapshot)
    if (this.#opening && snapshot.agents.some(a => a.id === this.#opening)) {
      const id = this.#opening
      this.#opening = ''
      this.#openBeside(id)
    }
  }

  /** A desk to open once the core has said who is here. */
  #opening = ''

  #openWhenHere(id: string) {
    if (this.snapshot?.agents.some(a => a.id === id)) this.#openBeside(id)
    else this.#opening = id
  }

  /** Open a desk's terminal beside the others, as the band does: nobody's pane is taken from under them. */
  #openBeside(id: string) {
    this.show(id, this.terminals || this.stowed.rows.length > 0)
  }

  /** True once for an agent that joined after the window opened: their desk plays the walk-in. */
  takeArrival(id: string): boolean {
    return this.#arrivals.delete(id)
  }

  /** Elapsed times only need a look now and then. */
  #syncClock() {
    const wanted = document.hidden || this.agents.length === 0 ? 0 : 20_000
    if (wanted === this.#clockMs) return
    clearInterval(this.#clock)
    this.#clockMs = wanted
    this.#clock = wanted ? setInterval(() => (this.now = Date.now()), wanted) : undefined
    if (wanted) this.now = Date.now()
  }

  // ── terminals ──────────────────────────────────────────────────────────

  /** A first guess at a terminal's size, for a program started before its pane exists. */
  #size(): [number, number] {
    const width = Math.max(480, window.innerWidth - (this.terminals ? this.floorWidth : FLOOR_WIDTH.terminals))
    const cols = Math.min(200, Math.max(60, Math.floor(width / 8.4) - 4))
    const rows = Math.min(70, Math.max(18, Math.floor((window.innerHeight - 120) / 19)))
    return [cols, rows]
  }

  /**
   * Show a desk's terminal: in the pane the keyboard is in, unless `beside` asks
   * for a pane of its own. Someone away is shown as they were left, not started:
   * their pane offers to carry on.
   */
  show(id: string, beside = false) {
    if (!this.agents.some(a => a.id === id)) return
    this.panel = null
    this.menu = null
    this.problem = ''
    this.#place(id, beside)
  }

  /**
   * Ctrl and a click on a desk, or Ctrl+Enter: their terminal beside the others. On a
   * desk whose terminal is there already, it is put away again; their program keeps running.
   */
  toggleBeside(id: string) {
    if (has(this.layout, id)) this.closePane(id)
    else this.show(id, true)
  }

  /** Start a desk's program again, or carry on its conversation, in its pane. */
  async wake(id: string) {
    const agent = this.agents.find(a => a.id === id)
    if (!agent || agent.running) return
    this.problem = ''
    try {
      await bridge.wake(id, ...this.#size())
    } catch (error) {
      this.problem = typeof error === 'string' ? error : `${agent.title} could not be started.`
    }
  }

  /** End a desk's program and start it again. */
  async restart(id: string) {
    const agent = this.agents.find(a => a.id === id)
    if (!agent) return
    this.problem = ''
    try {
      await bridge.restart(id, ...this.#size())
    } catch (error) {
      this.problem = typeof error === 'string' ? error : `${agent.title} could not be started again.`
    }
  }

  #place(id: string, beside: boolean) {
    let layout = this.layout
    // Back from the floor: the terminals that were open come back, with this one among them.
    if (layout.rows.length === 0 && this.stowed.rows.length > 0) layout = this.stowed
    if (!has(layout, id)) {
      const from = has(layout, this.focused) ? this.focused : (ids(layout).at(-1) ?? '')
      if (beside || !from) layout = open(layout, id, from || undefined)
      else {
        // A grid someone built: one of its panes making room is said, with a way back.
        if (ids(layout).length > 1) this.#sayMadeRoom(from, id, layout)
        layout = swap(layout, from, id)
      }
    }
    if (this.zoomed && this.zoomed !== id) this.zoomed = ''
    this.focused = id
    this.#lay(layout)
  }

  #sayMadeRoom(from: string, to: string, before: Layout) {
    const name = (id: string) => this.agents.find(a => a.id === id)?.title ?? 'A terminal'
    this.say(`${name(from)} made room for ${name(to)}. Ctrl and a click opens one beside the others.`, {
      label: 'Undo',
      run: () => {
        if (!this.agents.some(a => a.id === from)) return
        this.zoomed = ''
        this.focused = from
        this.#lay(only(before, id => this.agents.some(a => a.id === id) || this.jobs.some(j => j.id === id)))
      }
    })
  }

  /**
   * Put one terminal away. Its program keeps running; a finished install or update is forgotten.
   * The last one put away hands the keyboard to its desk on the floor, unless `toDesk` is false.
   */
  closePane(id: string, toDesk = true) {
    const list = ids(this.layout)
    const at = list.indexOf(id)
    if (at < 0) return
    if (isJob(id) && !this.jobs.find(j => j.id === id)?.running) {
      this.#pending.delete(id)
      bridge.forgetJob(id)
    }
    if (this.zoomed === id) this.zoomed = ''
    if (this.finding === id) this.finding = ''
    if (this.focused === id) this.focused = list[at + 1] ?? list[at - 1] ?? ''
    this.#lay(close(this.layout, id))
    // Every pane put away by hand: there is nothing to bring back.
    if (!this.terminals) this.stowed = EMPTY
    if (!isJob(id) && !this.leaving.has(id)) {
      const agent = this.agents.find(a => a.id === id)
      if (agent?.running) this.hint('put-away', `${agent.title} keeps working at their desk. To end their program: right-click the desk → Stop.`)
    }
    if (!this.terminals && toDesk) void tick().then(() => this.#focusDesk(id))
  }

  /** Back to the floor, or back to the terminals that were open before. */
  toggleTerminals() {
    if (this.terminals) {
      const last = this.focused
      this.stowed = this.layout
      this.zoomed = ''
      this.finding = ''
      this.#lay(EMPTY)
      void tick().then(() => this.#focusDesk(last))
    } else if (this.stowed.rows.length > 0) {
      const layout = this.stowed
      if (!has(layout, this.focused)) this.focused = ids(layout)[0] ?? ''
      this.panel = null
      this.#lay(layout)
    }
  }

  toggleZoom(id: string) {
    if (!has(this.layout, id) || ids(this.layout).length < 2) return
    this.zoomed = this.zoomed === id ? '' : id
    this.focused = id
    this.#report()
  }

  focusPane(id: string) {
    if (has(this.layout, id)) this.focused = id
  }

  /** Move the keyboard to the next pane, or the one before. */
  stepPane(by: 1 | -1) {
    const list = ids(this.layout)
    if (list.length < 2) return
    const at = Math.max(0, list.indexOf(this.focused))
    this.focused = list[(at + by + list.length) % list.length]!
    if (this.zoomed) this.zoomed = this.focused
  }

  /** Move the keyboard to the pane at this place in reading order, counting from one. */
  focusNth(n: number) {
    const id = ids(this.layout)[n - 1]
    if (!id) return
    this.focused = id
    if (this.zoomed) this.zoomed = id
  }

  /** Two panes change places, a pane dragged by its header onto another. */
  tradePanes(a: string, b: string) {
    const layout = trade(this.layout, a, b)
    if (layout !== this.layout) this.#lay(layout)
  }

  dragColumnEdge(row: number, index: number, at: number) {
    this.layout = moveColumnEdge(this.layout, row, index, at)
  }

  dragRowEdge(index: number, at: number) {
    this.layout = moveRowEdge(this.layout, index, at)
  }

  evenOut() {
    this.#lay(even(this.layout))
  }

  /** `keep` is false while an edge is being dragged: only where it is let go is remembered. */
  setFloorWidth(width: number, keep = true) {
    const most = Math.max(FLOOR_WIDTH.least, window.innerWidth - 420)
    this.floorWidth = Math.min(most, Math.max(FLOOR_WIDTH.least, Math.round(width)))
    if (keep) this.keepView()
  }

  setCloseQuits(on: boolean) {
    this.closeQuits = on
    bridge.setCloseQuits(on)
  }

  /**
   * Fetch the newer version and start the office again as it. The core asks first when
   * anyone is busy. If it cannot be had, that is said, with the way to get it by hand.
   */
  async updateOffice() {
    const newer = this.newer
    if (!newer || this.updating) return
    this.updating = true
    this.say(`Getting version ${newer.version}. The office starts again when it is in place.`, undefined, 120_000)
    try {
      if (!(await bridge.updateNow())) this.say('Not updated. It stays in the menu for when you are ready.')
    } catch (why) {
      this.say(`${typeof why === 'string' ? why : 'The update could not be had.'} It can also be downloaded from agentmoshpit.com.`, { label: 'Open the page', run: () => bridge.openPage('https://agentmoshpit.com/#download') }, 20_000)
    } finally {
      this.updating = false
    }
  }

  setCopyOnSelect(on: boolean) {
    this.copyOnSelect = on
    this.keepView()
  }

  /** The terminals' text, a size larger or smaller; zero for the usual size. */
  setFontSize(by: number) {
    const size = by === 0 ? FONT_SIZE.usual : this.fontSize + by
    this.fontSize = Math.min(FONT_SIZE.most, Math.max(FONT_SIZE.least, size))
    this.keepView()
  }

  #lay(layout: Layout) {
    this.layout = layout
    this.keepView()
    this.#report()
  }

  keepView() {
    try {
      const layout = this.terminals ? this.layout : this.stowed
      const view: View = { floor: this.floorWidth, layout, harness: this.lastHarness, open: this.terminals, font: this.fontSize, worktree: this.newDraft.worktree, editor: this.editorChoice, copySelect: this.copyOnSelect }
      localStorage.setItem(VIEW_KEY, JSON.stringify(view))
    } catch {
      // No storage: the view lasts until the window closes.
    }
  }

  /** Tell the core which terminals are in front of the user, so their flags come down. */
  #report() {
    const visible = this.zoomed ? [this.zoomed] : ids(this.layout)
    bridge.watch(document.hidden ? [] : visible)
  }

  // ── installing and updating ────────────────────────────────────────────

  /** Agents to start once the program they need has been installed, by the install's id. */
  #pending = new Map<string, { spec: NewAgentSpec; settled: number }>()

  /**
   * Install a program in a terminal of its own, beside the others. With `then`, the
   * agent it was wanted for is started as soon as it is there, in the same pane.
   */
  async install(harness: string, then?: NewAgentSpec) {
    this.problem = ''
    const id = await bridge.install(harness, ...this.#size())
    if (then) this.#pending.set(id, { spec: then, settled: 0 })
    this.panel = null
    this.#place(id, true)
  }

  async update(harness: string) {
    this.problem = ''
    try {
      const id = await bridge.update(harness, ...this.#size())
      this.panel = null
      this.#place(id, true)
    } catch (error) {
      this.problem = typeof error === 'string' ? error : 'That could not be updated.'
    }
  }

  /** An install that went well, and its program found: the agent takes over its pane. */
  #afterInstalls(snapshot: Snapshot) {
    for (const [id, waiting] of this.#pending) {
      const job: Job | undefined = snapshot.jobs.find(j => j.id === id)
      const kind = snapshot.harnesses.find(h => h.id === waiting.spec.harness)
      if (!job) {
        this.#pending.delete(id)
      } else if (job.ok === false) {
        this.#pending.delete(id)
        this.problem = `${job.harness_name} could not be installed. Its terminal says why.`
      } else if (job.ok && kind?.installed) {
        this.#pending.delete(id)
        void this.#startInPlaceOf(id, waiting.spec)
      } else if (job.ok && !waiting.settled) {
        // Installed, but not found yet: it is looked for again a moment after the end.
        waiting.settled = Date.now()
        setTimeout(() => {
          if (!this.#pending.has(id)) return
          this.#pending.delete(id)
          this.problem = `${job.harness_name} was installed but cannot be found. Open a new terminal and check that it runs, then start the agent again.`
        }, 8000)
      }
    }
  }

  async #startInPlaceOf(jobId: string, spec: NewAgentSpec) {
    try {
      const id = await bridge.newAgent(spec, ...this.#size())
      this.rememberFolder(spec.cwd)
      this.lastHarness = spec.harness
      if (this.focused === jobId) this.focused = id
      this.#lay(has(this.layout, jobId) ? swap(this.layout, jobId, id) : open(this.layout, id))
      bridge.forgetJob(jobId)
    } catch (error) {
      this.problem = typeof error === 'string' ? error : 'The agent could not be started.'
    }
  }

  openPrograms() {
    this.#rememberFocus()
    this.panel = { kind: 'programs' }
    // One installed since the window opened is offered too.
    this.findEditors()
  }

  // ── the editor file paths open in ──────────────────────────────────────

  findEditors() {
    void bridge.editors().then(
      list => (this.editors = list),
      () => {}
    )
  }

  /** Open file paths in this editor from now on. */
  setEditor(id: string) {
    this.editorChoice = id
    this.keepView()
  }

  /** Open a file a program printed, at its line, in the editor; or show it in its folder when there is none. */
  openFile(agent: string | null, path: string, line: number | null) {
    bridge.openFile(agent, path, line, this.editor?.id ?? '').then(
      how => {
        if (how === 'folder') this.say('No editor was found, so the file is shown in its folder. Cursor, VS Code, Windsurf, Antigravity and Zed are looked for.')
      },
      error => (this.problem = typeof error === 'string' ? error : `${path} could not be opened.`)
    )
  }

  /** Open a desk's folder in the editor. */
  openFolder(id: string) {
    const name = this.editor?.name ?? 'the editor'
    bridge.openFolder(id, this.editor?.id ?? '').catch(error => (this.problem = typeof error === 'string' ? error : `The folder could not be opened in ${name}.`))
  }

  openKeys() {
    this.#rememberFocus()
    this.panel = { kind: 'keys' }
  }

  openVoice() {
    this.#rememberFocus()
    this.panel = { kind: 'voice' }
  }

  // ── desks ──────────────────────────────────────────────────────────────

  async newAgent(spec: NewAgentSpec): Promise<void> {
    const id = await bridge.newAgent(spec, ...this.#size())
    this.rememberFolder(spec.cwd)
    this.lastHarness = spec.harness
    this.clearNewDraft()
    this.panel = null
    // A new agent gets a pane of their own: nobody's terminal is taken from under them.
    this.#place(id, true)
  }

  stop(id: string) {
    bridge.stop(id)
  }

  /** Whether taking this desk away would interrupt work, and so should be asked about first. */
  busy(id: string): boolean {
    const phase: Phase | undefined = this.agents.find(a => a.id === id)?.phase
    return phase === 'working' || phase === 'needs_you' || phase === 'starting'
  }

  /**
   * Take a desk away. It leaves the floor at once and can be brought back for a
   * few seconds; then its program is ended and the desk is gone.
   */
  remove(id: string) {
    const agent = this.agents.find(a => a.id === id)
    if (!agent) return
    // The keyboard goes to the desk beside it, as in a list when a row goes, so the arrow keys still work.
    const desks = [...document.querySelectorAll<HTMLElement>('.floor button[data-desk]')].map(desk => desk.dataset.desk ?? '')
    const at = desks.indexOf(id)
    const next = at < 0 ? '' : (desks[at + 1] ?? desks[at - 1] ?? '')
    const hadKeyboard = document.activeElement instanceof HTMLElement && document.activeElement.closest(`[data-seat="${CSS.escape(id)}"], .menu-sheet`) !== null
    this.menu = null
    this.leaving.add(id)
    this.closePane(id, false)
    if (next) this.homeDesk = next
    if (hadKeyboard || !this.terminals) void tick().then(() => this.#focusDesk(next))
    this.#removals.set(
      id,
      setTimeout(() => this.#removeFor(id), UNDO_MS)
    )
    this.say(`${agent.title} was taken off the floor.`, { label: 'Undo', run: () => this.#undoRemove(id) }, UNDO_MS)
  }

  #undoRemove(id: string) {
    clearTimeout(this.#removals.get(id))
    this.#removals.delete(id)
    this.leaving.delete(id)
    this.toast = null
    void tick().then(() => this.#focusDesk(id))
  }

  #removeFor(id: string) {
    this.#removals.delete(id)
    bridge.dismiss(id)
    this.leaving.delete(id)
  }

  #removeNow() {
    for (const [id, timer] of this.#removals) {
      clearTimeout(timer)
      this.#removeFor(id)
    }
  }

  /** Name a desk. An empty name gives it back the name it was given when it was seated. */
  rename(id: string, title: string) {
    bridge.rename(id, title.trim())
  }

  /** Bring the next desk in one of these states into view, and give it the keyboard. */
  findPhase(phases: Phase[]) {
    const desks = [...document.querySelectorAll<HTMLElement>('.floor button[data-desk]')].filter(desk => {
      const agent = this.agents.find(a => a.id === desk.dataset.desk)
      return agent !== undefined && phases.includes(agent.phase)
    })
    if (desks.length === 0) return
    const after = desks.findIndex(desk => desk.dataset.desk === this.#lastFound)
    const desk = desks[(after + 1) % desks.length]!
    this.#lastFound = desk.dataset.desk ?? ''
    desk.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'smooth' })
    desk.focus({ preventScroll: true })
    desk.classList.remove('found')
    void desk.offsetWidth
    desk.classList.add('found')
  }

  // ── what is said, and the small menus ──────────────────────────────────

  /** Say something at the foot of the window for a while. */
  say(text: string, action?: Toast['action'], ms = 6000) {
    clearTimeout(this.#toastTimer)
    this.toast = { text, action }
    this.#toastTimer = setTimeout(() => (this.toast = null), ms)
  }

  /** A hint said once, the first time it applies, and never again. */
  hint(key: string, text: string) {
    if (this.#hints.has(key)) return
    this.#hints.add(key)
    try {
      localStorage.setItem(HINTS_KEY, JSON.stringify([...this.#hints]))
    } catch {
      // No storage: the hint may be said again next time.
    }
    this.say(text, undefined, 9000)
  }

  openMenu(menu: Menu) {
    if (!this.menu) this.#rememberFocus()
    this.menu = menu
  }

  /** Where to open something about a desk: under its pane's title when that is on screen, else under the desk. */
  placeOf(id: string): { x: number; y: number } {
    const at = CSS.escape(id)
    const shown = this.zoomed === '' || this.zoomed === id
    const anchor =
      (shown ? document.querySelector(`.pane[data-pane="${at}"] h2`) : null) ??
      document.querySelector(`.seat[data-seat="${at}"] .name`) ??
      document.querySelector(`[data-desk="${at}"]`)
    const box = anchor?.getBoundingClientRect()
    return box && box.width > 0 ? { x: box.left, y: box.bottom + 6 } : { x: window.innerWidth / 2 - 130, y: 120 }
  }

  /** Open the box a desk is renamed in. */
  startRename(id: string, at?: { x: number; y: number }) {
    if (!this.agents.some(a => a.id === id)) return
    this.openMenu({ ...(at ?? this.placeOf(id)), items: [], renaming: id })
  }

  closeMenu() {
    if (!this.menu) return
    this.menu = null
    void tick().then(() => this.#giveFocusBack())
  }

  // ── the side panel ─────────────────────────────────────────────────────

  /** The New agent form, in a folder and with a program already chosen when they are given. */
  openNew(cwd = '', harness = '') {
    this.#rememberFocus()
    this.menu = null
    this.panel = { kind: 'new', cwd, harness }
  }

  closePanel() {
    const panel = this.panel
    this.panel = null
    if (panel) void tick().then(() => this.#giveFocusBack())
  }

  #rememberFocus() {
    const active = document.activeElement
    // A panel opened from inside a panel keeps the way back it already had.
    if (active instanceof HTMLElement && active.closest('.panel, .menu-sheet')) return
    this.#returnTo = active instanceof HTMLElement && active !== document.body ? active : null
  }

  /** Closing a panel takes whatever had focus with it; put focus back where it was. */
  #giveFocusBack() {
    const active = document.activeElement
    if (active && active !== document.body) return
    const back = this.#returnTo?.isConnected ? this.#returnTo : null
    ;(back ?? document.querySelector<HTMLElement>('.floor button[tabindex="0"]'))?.focus()
  }

  #focusDesk(id: string) {
    const desk = id ? document.querySelector<HTMLElement>(`[data-desk="${CSS.escape(id)}"]`) : null
    ;(desk ?? document.querySelector<HTMLElement>('.floor button[tabindex="0"]'))?.focus()
  }
}

export const office = new Office()
