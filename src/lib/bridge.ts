// The window's only way to the core. Inside the desktop app it calls the Rust
// side; in a plain browser it falls back to a pretend office (see demo.ts).
import { Channel, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { demoBridge } from './demo'
import type { Editor, NewAgentSpec, Newer, Settings, Snapshot, StartupProblem } from './types'
import type { VoiceModel, VoiceSettings, VoiceView } from './voice'

/**
 * What a terminal sends. `kept` is true for the first piece only: the screen as
 * it stood before the watching began, which must be drawn and not answered.
 */
export type OnTerminal = (bytes: Uint8Array, kept: boolean) => void

/** Files dragged over the window from the desktop, where they are in the page, and their paths once dropped. */
export interface Drop {
  kind: 'over' | 'drop' | 'leave'
  paths: string[]
  /** In the page's own pixels. */
  x: number
  y: number
}

export interface Bridge {
  /** True when this is the pretend office, with no programs behind it. */
  demo: boolean
  snapshot(): Promise<Snapshot>
  onSnapshot(fn: (snapshot: Snapshot) => void): () => void
  /** The tray's "New agent" item was chosen. */
  onNewAgent(fn: () => void): () => void
  takeNewAgent(): Promise<boolean>
  onStorageProblem(fn: (problem: StartupProblem) => void): () => void
  /** A notification was clicked: there is a desk to open (see `takeOpening`). */
  onOpenDesk(fn: () => void): () => void
  /** The desk a clicked notification asked for, once; null when there is none. */
  takeOpening(): Promise<string | null>
  /** Seat a new agent and start its program in a terminal of this size. Resolves with the desk's id. */
  newAgent(spec: NewAgentSpec, cols: number, rows: number): Promise<string>
  /** Install a program, or update it, in a terminal of its own. Resolves with that terminal's id. */
  install(harness: string, cols: number, rows: number): Promise<string>
  update(harness: string, cols: number, rows: number): Promise<string>
  /** Put away the terminal of an install or update for good. */
  forgetJob(job: string): void
  /** Make sure a desk's program is running, starting it again if it is not. */
  wake(agent: string, cols: number, rows: number): Promise<void>
  /** Follow a desk's terminal. Resolves with the way to stop following. */
  follow(agent: string, fn: OnTerminal): Promise<() => void>
  /** Keys for a desk's program. */
  type(agent: string, data: string): void
  resize(agent: string, cols: number, rows: number): void
  /** End a desk's program. The desk stays. */
  stop(agent: string): void
  /** End a desk's program and start it again, carrying on when it can. */
  restart(agent: string, cols: number, rows: number): Promise<void>
  /** Show a desk's folder in the file manager. */
  showFolder(agent: string): Promise<void>
  /** The editors on this computer that a file can be opened in, in the order they are offered. */
  editors(): Promise<Editor[]>
  /**
   * Open a file, at a line, in this editor (or the first one there is); or show it in its
   * folder when there is none. A path that is not whole is read from the desk's folder. Says which.
   */
  openFile(agent: string | null, path: string, line: number | null, editor: string): Promise<'editor' | 'folder'>
  /** Open a desk's folder in this editor, or the first one there is. */
  openFolder(agent: string, editor: string): Promise<void>
  /** Keep a pasted picture in a file. Resolves with its path. */
  savePastedImage(bytes: Uint8Array): Promise<string>
  /** What was wrong as the office started, if anything. */
  startupProblems(): Promise<StartupProblem[]>
  /** Files dragged over the window, and dropped. */
  onDrop(fn: (drop: Drop) => void): () => void
  /** Take a desk away, ending its program. */
  dismiss(agent: string): void
  rename(agent: string, title: string): void
  /** Which desks have their terminal on screen right now. */
  watch(agents: string[]): void
  /** Open a web page in the browser, if it is one that may be. */
  openPage(url: string): void
  pickFolder(): Promise<string | null>
  /** Quit the whole app, asking first if agents would be stopped. */
  quit(): void
  /** What the user chose that the core keeps. */
  settings(): Promise<Settings>
  /** Closing the window quits, or leaves the office in the tray. */
  setCloseQuits(on: boolean): void
  version(): Promise<string>
  /** The newer version of the office that is out, when one is; and each time that changes. */
  newer(): Promise<Newer | null>
  onNewer(fn: (newer: Newer | null) => void): () => void
  /**
   * Put the newer version in place and start the office again. The core asks first when
   * that would end someone's work: false is the user saying not now. It rejects with words
   * for the user when the update could not be had, and nothing was changed.
   */
  updateNow(): Promise<boolean>
  voiceView(): Promise<VoiceView>
  voiceConfig(settings: VoiceSettings): Promise<void>
  voiceStart(agent: string): Promise<void>
  voiceStop(): Promise<void>
  voiceCancel(): Promise<void>
  voiceDownload(model: VoiceModel): Promise<void>
  voiceCancelDownload(): Promise<void>
  voiceRemove(model: VoiceModel): Promise<void>
}

function subscribe<T>(name: string, fn: (payload: T) => void, ready?: () => void): () => void {
  let stop: (() => void) | undefined
  let stopped = false
  void listen<T>(name, event => fn(event.payload)).then(unlisten => {
    if (stopped) unlisten()
    else { stop = unlisten; ready?.() }
  })
  return () => {
    stopped = true
    stop?.()
  }
}

function tauriBridge(): Bridge {
  return {
    demo: false,
    snapshot: () => invoke<Snapshot>('snapshot'),
    onSnapshot: fn => subscribe<Snapshot>('office:snapshot', fn),
    onNewAgent: fn => subscribe<null>('office:new-agent', () => fn(), fn),
    takeNewAgent: () => invoke<boolean>('take_new_agent'),
    onStorageProblem: fn => subscribe<StartupProblem>('office:storage-problem', fn, () => {
      void invoke<StartupProblem[]>('startup_problems').then(problems => {
        const problem = problems.find(p => /desks\.json$/.test(p.file))
        if (problem) fn(problem)
      }, () => {})
    }),
    onOpenDesk: fn => subscribe<null>('office:open-desk', () => fn(), fn),
    takeOpening: () => invoke<string | null>('take_opening'),
    newAgent: (spec, cols, rows) => invoke<string>('new_agent', { spec, cols, rows }),
    install: (harness, cols, rows) => invoke<string>('install', { harness, cols, rows }),
    update: (harness, cols, rows) => invoke<string>('update', { harness, cols, rows }),
    forgetJob: job => void invoke('forget_job', { job }),
    wake: (agent, cols, rows) => invoke('wake', { agent, cols, rows }),
    follow: async (agent, fn) => {
      let kept = true
      let stopped = false
      const channel = new Channel<ArrayBuffer | number[]>()
      channel.onmessage = message => {
        if (stopped) return
        fn(message instanceof ArrayBuffer ? new Uint8Array(message) : Uint8Array.from(message), kept)
        kept = false
      }
      const token = await invoke<number>('term_attach', { agent, onData: channel })
      return () => {
        stopped = true
        void invoke('term_detach', { agent, token })
      }
    },
    type: (agent, data) => void invoke('term_write', { agent, data }).catch(() => {}),
    resize: (agent, cols, rows) => void invoke('term_resize', { agent, cols, rows }),
    stop: agent => void invoke('stop', { agent }),
    restart: (agent, cols, rows) => invoke('restart', { agent, cols, rows }),
    showFolder: agent => invoke('show_folder', { agent }),
    editors: () => invoke<Editor[]>('editors'),
    openFile: (agent, path, line, editor) => invoke<'editor' | 'folder'>('open_file', { agent, path, line, editor: editor || null }),
    openFolder: (agent, editor) => invoke('open_folder', { agent, editor: editor || null }),
    savePastedImage: bytes => invoke<string>('save_pasted_image', bytes),
    startupProblems: () => invoke<StartupProblem[]>('startup_problems'),
    onDrop: fn => {
      let stop: (() => void) | undefined
      let stopped = false
      void getCurrentWebview()
        .onDragDropEvent(event => {
          const drop = event.payload
          if (drop.type === 'leave') return fn({ kind: 'leave', paths: [], x: 0, y: 0 })
          // The webview says where in physical pixels; the page thinks in its own.
          const scale = window.devicePixelRatio || 1
          const [x, y] = [drop.position.x / scale, drop.position.y / scale]
          fn({ kind: drop.type === 'drop' ? 'drop' : 'over', paths: drop.type === 'over' ? [] : drop.paths, x, y })
        })
        .then(unlisten => {
          if (stopped) unlisten()
          else stop = unlisten
        })
      return () => {
        stopped = true
        stop?.()
      }
    },
    dismiss: agent => void invoke('dismiss', { agent }),
    rename: (agent, title) => void invoke('rename', { agent, title }),
    watch: agents => void invoke('watch', { agents }),
    openPage: url => void invoke('open_page', { url }),
    pickFolder: () => invoke<string | null>('pick_folder'),
    quit: () => void invoke('quit'),
    settings: () => invoke<Settings>('settings'),
    setCloseQuits: on => void invoke('set_close_quits', { on }),
    version: () => invoke<string>('app_version'),
    newer: () => invoke<Newer | null>('newer_version'),
    onNewer: fn => subscribe<Newer | null>('office:newer', fn),
    updateNow: () => invoke<boolean>('update_now'),
    voiceView: () => invoke<VoiceView>('voice_view'),
    voiceConfig: settings => invoke('voice_config', { settings }),
    voiceStart: agent => invoke('voice_start', { agent }),
    voiceStop: () => invoke('voice_stop'),
    voiceCancel: () => invoke('voice_cancel'),
    voiceDownload: model => invoke('voice_download', { model }),
    voiceCancelDownload: () => invoke('voice_cancel_download'),
    voiceRemove: model => invoke('voice_remove', { model })
  }
}

export const bridge: Bridge = '__TAURI_INTERNALS__' in window ? tauriBridge() : demoBridge()
