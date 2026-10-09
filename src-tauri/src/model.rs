//! What the window is told about the office. Everything here is plain data:
//! the window draws it and never starts a program itself.

use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch.
pub type Millis = u64;

pub fn now_ms() -> Millis {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as Millis)
        .unwrap_or(0)
}

/// The one status a person needs to read at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// The program has been started and has not drawn anything yet.
    Starting,
    Working,
    /// Stopped on a question only the user can answer, in its terminal.
    NeedsYou,
    /// Finished a stretch of work that nobody has looked at yet.
    Done,
    /// Running, with nothing to do.
    Idle,
    /// The program could not be started, or ended with an error.
    Failed,
    /// Not running. Its desk shows where it left off, and offers to start it again.
    Asleep,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentView {
    pub id: String,
    pub title: String,
    pub phase: Phase,
    /// One line about what is going on, when there is something to say. Often empty.
    pub activity: String,
    /// Which program this is: its id in the table, its name, and the short word for a desk's tag.
    pub harness: String,
    pub harness_name: String,
    pub harness_tag: String,
    pub repo: String,
    /// Canonical repository root, shared by worktrees; a folder path outside git.
    pub project: String,
    pub branch: String,
    pub cwd: String,
    /// When the current phase began.
    pub since_ms: Millis,
    /// Stable number the window turns into a face and a shirt.
    pub look: u32,
    /// Whether its program is running right now.
    pub running: bool,
    /// Whether a stopped one carries on where it left off, or starts afresh.
    pub resumable: bool,
    pub resume_note: String,
    /// Its terminal printed something since the user last had it in front of them.
    pub unread: bool,
}

/// One program an agent can be, and whether it is on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HarnessView {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub installed: bool,
    /// What `--version` printed. Empty until it has answered, or when it did not.
    pub version: String,
    /// Whether the first thing to do can be handed over when it starts.
    pub takes_task: bool,
    /// Whether it can make its own separate copy of the project.
    pub worktree: bool,
    /// The newest version published, when that has been asked. Empty otherwise.
    pub latest: String,
    /// Installed, and older than `latest`.
    pub outdated: bool,
    /// What runs to install it, and to update it, as a person would type it.
    /// Empty when the office does not know a way.
    pub install_line: String,
    pub update_line: String,
}

/// An install or an update, running in a terminal of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobView {
    pub id: String,
    pub harness: String,
    pub harness_name: String,
    /// `install` or `update`.
    pub kind: String,
    pub running: bool,
    /// How it ended. `None` while it runs.
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Snapshot {
    pub agents: Vec<AgentView>,
    pub harnesses: Vec<HarnessView>,
    pub jobs: Vec<JobView>,
    pub now_ms: Millis,
}

/// What the window asks for when someone presses Start agent.
#[derive(Debug, Clone, Deserialize)]
pub struct NewAgent {
    /// The id of a program in the table.
    pub harness: String,
    /// Folder the agent works in.
    pub cwd: String,
    /// The first thing to ask it. May be empty.
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub title: String,
    /// Ask the program to work on a git worktree of its own.
    #[serde(default)]
    pub worktree: bool,
}
