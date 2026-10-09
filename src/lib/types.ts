// What the core tells the window. Mirrors `src-tauri/src/model.rs`.

export type Phase = 'starting' | 'working' | 'needs_you' | 'done' | 'idle' | 'failed' | 'asleep'

export interface Agent {
  id: string
  title: string
  phase: Phase
  /** One line about what is going on, when the program said. Often empty. */
  activity: string
  /** Which program this is: its id in the table, its name, and the short word for a desk. */
  harness: string
  harness_name: string
  harness_tag: string
  repo: string
  branch: string
  cwd: string
  since_ms: number
  look: number
  /** Whether its program is running right now. */
  running: boolean
  /** Whether a stopped one carries on where it left off, or starts afresh. */
  resumable: boolean
  /** Its terminal printed something since you last had it in front of you. */
  unread: boolean
}

/** One program an agent can be, and whether it is on this computer. */
export interface Harness {
  id: string
  name: string
  tag: string
  installed: boolean
  /** Empty until it has answered, or when it did not. */
  version: string
  /** Whether the first thing to do can be handed over when it starts. */
  takes_task: boolean
  /** Whether it can make its own separate copy of the project. */
  worktree: boolean
  /** The newest version published, once npm has been asked. Empty otherwise. */
  latest: string
  /** Installed, and older than `latest`. */
  outdated: boolean
  /** What runs to install it, and to update it, as typed. Empty when there is no way. */
  install_line: string
  update_line: string
}

/** An install or an update, running in a terminal of its own. Its id starts with `job-`. */
export interface Job {
  id: string
  harness: string
  harness_name: string
  kind: 'install' | 'update'
  running: boolean
  /** How it ended; null while it runs. */
  ok: boolean | null
}

export interface Snapshot {
  agents: Agent[]
  harnesses: Harness[]
  jobs: Job[]
  now_ms: number
}

/** Something wrong with a file the user wrote (`harnesses.json`), found as the office started. */
export interface StartupProblem {
  text: string
  /** The file to open to put it right, and the line when one is known. */
  file: string
  line: number | null
}

/** What the user chose that the core keeps. */
export interface Settings {
  /** Closing the window quits the office instead of leaving it in the tray. */
  close_quits: boolean
}

/** An editor on this computer that file paths can be opened in. */
export interface Editor {
  /** `cursor`, `code`, `windsurf`, `antigravity` or `zed`. */
  id: string
  name: string
}

export interface NewAgentSpec {
  harness: string
  cwd: string
  prompt: string
  title?: string
  worktree?: boolean
}
