// What the core tells the window. Mirrors `src-tauri/src/model.rs`.

export type Phase = 'starting' | 'working' | 'quiet' | 'needs_you' | 'done' | 'idle' | 'failed' | 'asleep'

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
  project: string
  repo: string
  branch: string
  cwd: string
  since_ms: number
  look: number
  /** Whether its program is running right now. */
  running: boolean
  /** Whether a stopped one carries on where it left off, or starts afresh. */
  resumable: boolean
  resume_scope?: 'none' | 'desk' | 'folder' | 'latest'
  resume_note: string
  /** Its terminal printed something since you last had it in front of you. */
  unread: boolean
  /** Explicit settings at launch; the program can change them in its terminal. */
  launch?: LaunchOptions
}

/** One program an agent can be, and whether it is on this computer. */
export interface Harness {
  id: string
  name: string
  tag: string
  installed: boolean
  launch?: 'none' | 'claude' | 'codex'
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

/** A newer version of the office itself that is out. */
export interface Newer {
  version: string
}

/** What the user chose that the core keeps. */
export interface Settings {
  /** Closing the window quits the office instead of leaving it in the tray. */
  close_quits: boolean
  voice: import('./voice').VoiceSettings
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
  launch?: LaunchOptions
}

export interface LaunchOptions {
  model?: string
  effort?: string
  permission?: string
  sandbox?: string
  approval?: string
  profile?: string
  search?: boolean | null
  additional_dirs?: string[]
  allowed_tools?: string[]
  disallowed_tools?: string[]
  chrome?: boolean | null
  instructions?: string
}

export interface ModelChoice {
  id: string
  name: string
  description: string
  efforts: string[]
  resolved: string
  auto_mode: boolean | null
}

export interface PermissionChoice {
  id: string
  name: string
  description: string
}

export interface ModelCatalog {
  models: ModelChoice[]
  permissions: PermissionChoice[]
  features: string[]
  fetched_ms: number
  source: string
  problem: string
}
