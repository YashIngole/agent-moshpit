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

/** Where to get what a program needs before the office can install it (`install_needs`). */
export const WHERE_TO_GET: Record<string, string> = { 'Node.js': 'https://nodejs.org/en/download' }

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
  /** What has to be on this computer before the office can install it, and is not: `Node.js`, when it is installed with npm and npm was not found. Empty otherwise. */
  install_needs: string
  /** Whether someone is signed in to it, as it says itself. Null until it has been asked, and for a program that cannot say. */
  signed_in: boolean | null
  /** Who is signed in, or that nobody is, in a few words. Empty when not known. */
  account: string
  /** What runs to sign in, as typed. Empty when the office knows no way. */
  login_line: string
}

/** An install, an update or a sign-in, running in a terminal of its own. Its id starts with `job-`. */
export interface Job {
  id: string
  harness: string
  harness_name: string
  kind: 'install' | 'update' | 'signin'
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
  /** How something new from one agent is brought to another's attention. */
  messages: import('./messages').MessageDelivery
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
  /** Plugins switched on or off for this desk alone, by name. Only those changed from the program's own settings. */
  plugins?: Record<string, boolean>
  /** Further words for the program, as typed, after every setting above. */
  extra_args?: string[]
}

/** A plugin the program has installed, and whether it is on in its own settings. */
export interface PluginChoice {
  id: string
  enabled: boolean
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
  /** The plugins it has installed, when it can be told which to use for one run. */
  plugins?: PluginChoice[]
  features: string[]
  fetched_ms: number
  source: string
  problem: string
}
