//! The programs an agent can be: Claude Code, Codex, and the rest of the table.
//!
//! The office knows very little about each one, on purpose: what it is called,
//! what to type to start it, and how to hand it a task. Everything else is the
//! program's own business and is seen through its terminal. A program that
//! changes how it looks or what it can do therefore changes nothing here, and a
//! new one is a new row: built in below, or in `harnesses.json` beside the desks.
//!
//! A row may also say how the program is installed and updated: through npm,
//! when it is published there, or by the program's own update command. Those run
//! in a terminal in the window, where the user sees every line of them.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How a program is given the first thing to do.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskArg {
    /// As the last word on its command line.
    Last,
    /// After this flag.
    Flag(String),
    /// Not on the command line. The user types it in the terminal.
    #[default]
    None,
}

/// Where the office learns whether a program is working, idle or waiting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusFrom {
    /// The files Claude Code keeps about its own running sessions.
    ClaudeSessions,
    /// What its terminal is doing: printing, quiet, or ringing the bell. Works for anything.
    #[default]
    Activity,
}

/// How the office finds out what a program calls the conversation it just began.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionFrom {
    /// The office picks the id and passes it after `session_arg`.
    Given,
    /// Resolve the session id reported by this process's terminal title.
    CodexRollouts,
    /// It is not known. Resuming uses whatever `resume` says without an id.
    #[default]
    Unknown,
}

/// Which startup trust dialog the office recognizes on the actual terminal screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustFrom {
    /// Claude Code's visible folder-trust choices.
    Claude,
    /// Codex's visible folder-trust choices.
    Codex,
    /// It does not ask, or the office cannot tell.
    #[default]
    None,
}

/// Where the office learns what a conversation is about, to name a desk nobody has named.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicFrom {
    /// The few words the program puts in its terminal's title once it has something to do.
    TerminalTitle,
    /// It says nothing the office can read. The desk keeps the name it was seated with.
    #[default]
    None,
}

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Harness {
    pub id: String,
    /// "Claude Code".
    pub name: String,
    /// The short word on a desk: "Claude".
    pub tag: String,
    /// What is typed to start it: "claude".
    pub program: String,
    /// The adapter for per-desk launch settings and live model discovery.
    /// Custom rows can opt in without depending on their id or program name.
    #[serde(default)]
    pub launch: crate::launch::Provider,
    /// Words passed every time, before anything else.
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub task: TaskArg,
    /// Words that make it carry on an earlier conversation. `{session}` stands
    /// for its id. Empty when it cannot.
    #[serde(default)]
    pub resume: Vec<String>,
    #[serde(default)]
    pub session: SessionFrom,
    /// The flag that sets a new conversation's id, when `session` is `given`.
    #[serde(default)]
    pub session_arg: String,
    /// The flag that names a conversation. Empty when there is none.
    #[serde(default)]
    pub name_arg: String,
    /// Words that make it work on a git worktree of its own. Empty when it cannot.
    #[serde(default)]
    pub worktree: Vec<String>,
    #[serde(default)]
    pub status: StatusFrom,
    /// Which visible startup trust dialog can be recognized.
    #[serde(default)]
    pub trust: TrustFrom,
    /// Where the topic of a conversation can be read.
    #[serde(default)]
    pub topic: TopicFrom,
    /// Its package on npm, which installs it, updates it when it has no way of
    /// its own, and says which version is the latest. Empty when it is not there.
    #[serde(default)]
    pub package: String,
    /// Words that make the program update itself. Empty: npm does it, when it can.
    #[serde(default)]
    pub update: Vec<String>,
    /// A command that installs it, program first, for a row someone added by hand
    /// that npm does not install. Built-in rows leave it empty.
    #[serde(default)]
    pub install: Vec<String>,
    /// Words that make the program sign someone in: `claude auth login`. Empty when
    /// it has no such command, and signing in is done in its own terminal.
    #[serde(default)]
    pub login: Vec<String>,
    /// Words that make it say who is signed in, asking nothing: `claude auth status`.
    /// Empty when it cannot say.
    #[serde(default)]
    pub account: Vec<String>,
}

fn row(id: &str, name: &str, tag: &str, program: &str) -> Harness {
    Harness {
        id: id.into(),
        name: name.into(),
        tag: tag.into(),
        program: program.into(),
        launch: crate::launch::Provider::None,
        args: Vec::new(),
        task: TaskArg::None,
        resume: Vec::new(),
        session: SessionFrom::Unknown,
        session_arg: String::new(),
        name_arg: String::new(),
        worktree: Vec::new(),
        status: StatusFrom::Activity,
        trust: TrustFrom::None,
        topic: TopicFrom::None,
        package: String::new(),
        update: Vec::new(),
        install: Vec::new(),
        login: Vec::new(),
        account: Vec::new(),
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|w| (*w).to_string()).collect()
}

/// The programs the office knows without being told.
///
/// Claude Code, Codex, Hermes and Gemini CLI were read from each program's own
/// `--help` on 2026-10-08 (Claude Code 2.1.290, Codex 0.160.0, Hermes 0.21.2,
/// Gemini CLI 0.2.1). Antigravity CLI, which took Gemini CLI's place for personal
/// Google accounts on 2026-06-18, was read from Google's documentation on
/// 2026-10-09 (antigravity.google/docs/cli) and has not been run by the office's
/// makers. The rest are only known by name: they start in their folder and the
/// task is typed in their terminal, which is right for any program whatever its
/// flags are.
///
/// The npm packages were each looked up on the registry the same day. A program
/// that is installed some other way (a script piped to a shell, pip, a download)
/// has no package here: the office does not run such things for the user.
pub fn built_in() -> Vec<Harness> {
    vec![
        Harness {
            launch: crate::launch::Provider::Claude,
            task: TaskArg::Last,
            resume: words(&["--resume", "{session}"]),
            session: SessionFrom::Given,
            session_arg: "--session-id".into(),
            name_arg: "--name".into(),
            worktree: words(&["--worktree"]),
            status: StatusFrom::ClaudeSessions,
            trust: TrustFrom::Claude,
            topic: TopicFrom::TerminalTitle,
            package: "@anthropic-ai/claude-code".into(),
            update: words(&["update"]),
            login: words(&["auth", "login"]),
            account: words(&["auth", "status", "--json"]),
            ..row("claude", "Claude Code", "Claude", "claude")
        },
        Harness {
            // Codex rings its terminal when it wants an answer or has finished, if asked to,
            // and titles it with its session's id and then what the conversation is about.
            launch: crate::launch::Provider::Codex,
            args: words(&["-c", "tui.notifications=true", "-c", "tui.terminal_title=[\"session-id\",\"thread-title\"]"]),
            task: TaskArg::Last,
            resume: words(&["resume", "{session}"]),
            session: SessionFrom::CodexRollouts,
            worktree: words(&["--worktree"]),
            trust: TrustFrom::Codex,
            topic: TopicFrom::TerminalTitle,
            package: "@openai/codex".into(),
            update: words(&["update"]),
            login: words(&["login"]),
            account: words(&["login", "status"]),
            ..row("codex", "Codex", "Codex", "codex")
        },
        Harness {
            task: TaskArg::Flag("--prompt-interactive".into()),
            // Its latest conversation in this folder: it keeps them by folder.
            resume: words(&["--continue"]),
            // Installed by a script from Google, not from npm, so the office does not install it.
            update: words(&["update"]),
            ..row("antigravity", "Antigravity CLI", "Antigravity", "agy")
        },
        Harness {
            resume: words(&["--continue"]),
            worktree: words(&["--worktree"]),
            update: words(&["update"]),
            ..row("hermes", "Hermes", "Hermes", "hermes")
        },
        // Still there for those it still serves: a paid API key, or an enterprise licence.
        Harness {
            task: TaskArg::Flag("--prompt-interactive".into()),
            package: "@google/gemini-cli".into(),
            ..row("gemini", "Gemini CLI", "Gemini", "gemini")
        },
        Harness { package: "opencode-ai".into(), ..row("opencode", "OpenCode", "OpenCode", "opencode") },
        row("cursor", "Cursor CLI", "Cursor", "cursor-agent"),
        Harness { package: "@github/copilot".into(), ..row("copilot", "Copilot CLI", "Copilot", "copilot") },
        Harness { package: "@sourcegraph/amp".into(), ..row("amp", "Amp", "Amp", "amp") },
        row("aider", "Aider", "Aider", "aider"),
        row("goose", "Goose", "Goose", "goose"),
        Harness { package: "@qwen-code/qwen-code".into(), ..row("qwen", "Qwen Code", "Qwen", "qwen") },
        Harness { package: "@charmland/crush".into(), ..row("crush", "Crush", "Crush", "crush") },
        row("droid", "Droid", "Droid", "droid"),
    ]
}

/// Something wrong with a file the user wrote, said so they can fix it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    pub text: String,
    /// The file to open to put it right, and the line, when one is known.
    pub file: String,
    pub line: Option<u32>,
}

/// The built-in table with the user's own rows laid over it: a row with a known
/// id replaces that program, any other is added at the end.
/// Also returned: what was wrong with the user's own file, said so they can fix it.
/// A file that is not there is not a problem.
pub fn table_checked(user_file: &Path) -> (Vec<Harness>, Vec<Problem>) {
    let mut all = built_in();
    let mut problems = Vec::new();
    let Ok(text) = std::fs::read_to_string(user_file) else {
        return (all, problems);
    };
    let name = user_file.file_name().map_or_else(|| "harnesses.json".to_string(), |n| n.to_string_lossy().into_owned());
    let file = user_file.to_string_lossy().into_owned();
    let rows = match serde_json::from_str::<Vec<Harness>>(&text) {
        Ok(rows) => rows,
        Err(why) => {
            // The usual slip on Windows is a path with single backslashes: said only when that is it.
            let tip = if why.to_string().contains("escape") { " In JSON a \\ is written \\\\." } else { "" };
            let text = format!("{name} could not be read ({why}), so the programs in it are not offered.{tip} It is {file}.");
            problems.push(Problem { text, file, line: u32::try_from(why.line()).ok().filter(|n| *n > 0) });
            return (all, problems);
        }
    };
    for mine in rows {
        if !usable(&mine) {
            let text = format!(
                "In {name}, the row for \"{}\" was skipped: its id may use only letters, digits, - and _, and its program must be a plain name or path, without \" & | < > ^ % ; ` or $. It is {file}.",
                mine.id
            );
            problems.push(Problem { text, file: file.clone(), line: None });
            continue;
        }
        match all.iter_mut().find(|h| h.id == mine.id) {
            Some(known) => *known = mine,
            None => all.push(mine),
        }
    }
    (all, problems)
}

/// A row someone wrote by hand has to name a program plainly: a bare name or a
/// path, never a line for a shell to read.
fn usable(h: &Harness) -> bool {
    let plain = |s: &str| !s.trim().is_empty() && !s.chars().any(|c| c.is_control() || matches!(c, '"' | '&' | '|' | '<' | '>' | '^' | '%' | ';' | '`' | '$'));
    plain(&h.id)
        && plain(&h.name)
        && plain(&h.program)
        && h.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && (h.package.is_empty() || package_name(&h.package))
}

/// What npm accepts as a package's name: `name` or `@scope/name`, in lower case.
fn package_name(name: &str) -> bool {
    let bare = name.strip_prefix('@').and_then(|rest| rest.split_once('/')).map_or(name, |(_, rest)| rest);
    let part = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'));
    let scope_ok = name.strip_prefix('@').is_none_or(|rest| rest.split_once('/').is_some_and(|(scope, _)| part(scope)));
    part(bare) && scope_ok && !name.starts_with('.') && !bare.starts_with('.')
}

// ── installing and updating ────────────────────────────────────────────────

/// What runs to install or update a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runner {
    /// The program itself, with these words.
    Itself(Vec<String>),
    /// npm, with these words.
    Npm(Vec<String>),
    /// Another program, named first, with the words after it.
    Command(Vec<String>),
}

impl Runner {
    /// The command as a person would type it, to show before it runs.
    pub fn line(&self, program: &str) -> String {
        let (first, rest): (&str, &[String]) = match self {
            Runner::Itself(words) => (program, words),
            Runner::Npm(words) => ("npm", words),
            Runner::Command(words) => (words.first().map_or("", String::as_str), words.get(1..).unwrap_or(&[])),
        };
        let quote = |w: &str| {
            if w.is_empty() || w.contains(' ') {
                format!("\"{w}\"")
            } else {
                w.to_string()
            }
        };
        std::iter::once(first.to_string()).chain(rest.iter().map(|w| quote(w))).collect::<Vec<_>>().join(" ")
    }
}

/// How a program that is not on this computer is put there, if the office can.
pub fn install_runner(h: &Harness) -> Option<Runner> {
    if !h.install.is_empty() {
        return Some(Runner::Command(h.install.clone()));
    }
    (!h.package.is_empty()).then(|| Runner::Npm(vec!["install".into(), "-g".into(), h.package.clone()]))
}

/// How a program brings itself up to date, if the office knows a way.
pub fn update_runner(h: &Harness) -> Option<Runner> {
    if !h.update.is_empty() {
        return Some(Runner::Itself(h.update.clone()));
    }
    (!h.package.is_empty()).then(|| Runner::Npm(vec!["install".into(), "-g".into(), format!("{}@latest", h.package)]))
}

/// How a program signs someone in, if it has a command for that.
pub fn login_runner(h: &Harness) -> Option<Runner> {
    (!h.login.is_empty()).then(|| Runner::Itself(h.login.clone()))
}

/// The words for one of a program's own commands, as it has to be given them. A row
/// may name a program that runs another (`node`, with a script as its first word): the
/// words before its first flag are that other program, and come before any command.
pub fn own_words(h: &Harness, command: &[String]) -> Vec<String> {
    h.args.iter().take_while(|word| !word.starts_with('-')).chain(command).cloned().collect()
}

/// The newest version of a package, as npm says. Empty when npm is not here or does not answer.
pub fn latest(npm: &Found, package: &str) -> String {
    if !package_name(package) {
        return String::new();
    }
    let said = ask(npm, &["view".to_string(), package.to_string(), "version".to_string()], 20);
    let line = said.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    // npm prints the bare version; anything else is an error message.
    if line.chars().next().is_some_and(|c| c.is_ascii_digit()) && !line.contains(' ') {
        line.to_string()
    } else {
        String::new()
    }
}

/// Whether `latest` is a newer version than `installed`, number by number.
/// When either is not a version, nothing is said to be newer.
pub fn newer(installed: &str, latest: &str) -> bool {
    let numbers = |v: &str| -> Option<Vec<u64>> {
        let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
        let parts: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
        parts.filter(|p| !p.is_empty())
    };
    match (numbers(installed), numbers(latest)) {
        (Some(have), Some(out)) => {
            for i in 0..have.len().max(out.len()) {
                let (a, b) = (have.get(i).copied().unwrap_or(0), out.get(i).copied().unwrap_or(0));
                if a != b {
                    return b > a;
                }
            }
            false
        }
        _ => false,
    }
}

// ── what to type ───────────────────────────────────────────────────────────

/// The words that start a new conversation.
pub fn start_args(h: &Harness, session: Option<&str>, title: &str, task: &str, worktree: bool) -> Vec<String> {
    let mut args = h.args.clone();
    if let (SessionFrom::Given, Some(id)) = (h.session, session) {
        if !h.session_arg.is_empty() {
            args.push(h.session_arg.clone());
            args.push(id.to_string());
        }
    }
    if !h.name_arg.is_empty() && !title.trim().is_empty() {
        args.push(h.name_arg.clone());
        args.push(title.trim().to_string());
    }
    if worktree {
        args.extend(h.worktree.iter().cloned());
    }
    let task = task.trim();
    if !task.is_empty() {
        match &h.task {
            TaskArg::Last => args.push(task.to_string()),
            TaskArg::Flag(flag) => {
                args.push(flag.clone());
                args.push(task.to_string());
            }
            TaskArg::None => {}
        }
    }
    args
}

/// The words that carry on an earlier conversation, or `None` when this program
/// cannot, or needs an id that is not known.
pub fn resume_args(h: &Harness, session: Option<&str>) -> Option<Vec<String>> {
    if h.resume.is_empty() {
        return None;
    }
    let needs_id = h.resume.iter().any(|w| w.contains("{session}"));
    let id = session.filter(|s| !s.is_empty());
    if needs_id && id.is_none() {
        return None;
    }
    // Codex takes its settings before the word `resume`; every other word order is the same.
    let mut args = h.args.clone();
    args.extend(h.resume.iter().map(|w| w.replace("{session}", id.unwrap_or(""))));
    Some(args)
}

/// Whether a stopped desk of this kind carries on, given what is known about it.
pub fn can_resume(h: &Harness, session: Option<&str>) -> bool {
    resume_args(h, session).is_some()
}

/// What a resume action can actually promise about the conversation it selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeScope {
    None,
    Desk,
    Folder,
    Latest,
}

pub fn resume_scope(h: &Harness, session: Option<&str>) -> ResumeScope {
    if !can_resume(h, session) {
        ResumeScope::None
    } else if h.resume.iter().any(|word| word.contains("{session}")) {
        ResumeScope::Desk
    } else if h.id == "antigravity" {
        ResumeScope::Folder
    } else {
        ResumeScope::Latest
    }
}

// ── where it is ────────────────────────────────────────────────────────────

/// A program found on this computer, as it has to be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The file to run.
    pub program: String,
    /// Words that come before the program's own: the script, when the file is `node`.
    pub before: Vec<String>,
    /// True when it can only be started through `cmd.exe`, which reads some
    /// characters as commands. Such a program is only ever given plain words.
    pub through_shell: bool,
}

/// How long a list of folders is gone by before it is made again. A program installed
/// while the office is open, Node.js above all, is found the next time anything is
/// looked for, without restarting the office.
const FOLDERS_FOR: Duration = Duration::from_secs(3);

/// The folders last listed, and when.
static FOLDERS: Mutex<Option<(Instant, Arc<Vec<PathBuf>>)>> = Mutex::new(None);

/// The PATH the user's login shell sets up, once it has been asked. See `look_again`.
#[cfg(unix)]
static SHELL_PATH: Mutex<Option<Option<std::ffi::OsString>>> = Mutex::new(None);

/// When the login shell was last asked again, which starts it: not more than once a minute.
#[cfg(unix)]
static SHELL_ASKED: Mutex<Option<Instant>> = Mutex::new(None);
#[cfg(unix)]
const SHELL_EVERY: Duration = Duration::from_secs(60);

/// Every folder a program might be in: the PATH this app was given, the PATH a
/// login shell would have (an app started from a dock or a menu is given a much
/// shorter one), the PATH Windows keeps for programs started from now on (an
/// installer adds to that one, never to a running app's), and the places installers
/// usually use.
fn folders() -> Arc<Vec<PathBuf>> {
    let mut kept = FOLDERS.lock().unwrap();
    if let Some((at, all)) = kept.as_ref() {
        if at.elapsed() < FOLDERS_FOR {
            return all.clone();
        }
    }
    let all = Arc::new(list_folders());
    *kept = Some((Instant::now(), all.clone()));
    all
}

fn list_folders() -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = Vec::new();
    let mut add = |dir: PathBuf| {
        if !dir.as_os_str().is_empty() && !all.contains(&dir) {
            all.push(dir);
        }
    };
    if let Some(path) = std::env::var_os("PATH") {
        std::env::split_paths(&path).for_each(&mut add);
    }
    #[cfg(unix)]
    {
        let mut asked = SHELL_PATH.lock().unwrap();
        if let Some(path) = asked.get_or_insert_with(login_shell_path).as_ref() {
            std::env::split_paths(path).for_each(&mut add);
        }
    }
    #[cfg(windows)]
    for path in windows_path::now() {
        std::env::split_paths(&path).for_each(&mut add);
    }
    if let Some(home) = home_dir() {
        for rest in [".local/bin", ".npm-global/bin", ".bun/bin", ".cargo/bin", ".opencode/bin", ".hermes/bin"] {
            add(home.join(rest));
        }
        #[cfg(windows)]
        for rest in ["AppData/Roaming/npm", "AppData/Local/agy/bin", "AppData/Local/hermes/bin", "AppData/Local/Programs/hermes/bin", "scoop/shims", "AppData/Local/Microsoft/WinGet/Links", "AppData/Local/Volta/bin"] {
            add(home.join(rest));
        }
    }
    // Where Node.js is put by its own installer, whatever the PATH says yet.
    #[cfg(windows)]
    for root in ["ProgramFiles", "ProgramW6432"] {
        if let Some(root) = std::env::var_os(root).filter(|r| !r.is_empty()) {
            add(PathBuf::from(root).join("nodejs"));
        }
    }
    #[cfg(unix)]
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/home/linuxbrew/.linuxbrew/bin"] {
        add(PathBuf::from(dir));
    }
    all
}

/// Forget where programs were thought to be: the next search reads the PATH anew.
/// With `shell`, on Unix, the login shell is asked for its PATH again as well, when a
/// minute has gone by since the last time. That starts the shell and waits two seconds
/// at most, so it is for a thread that can wait, and for when somebody is looking for a
/// program; a shell that does not answer leaves what it said before.
pub fn look_again(shell: bool) {
    #[cfg(unix)]
    if shell {
        let mut asked = SHELL_ASKED.lock().unwrap();
        if !matches!(*asked, Some(at) if at.elapsed() < SHELL_EVERY) {
            *asked = Some(Instant::now());
            if let Some(path) = login_shell_path() {
                *SHELL_PATH.lock().unwrap() = Some(Some(path));
            }
        }
    }
    #[cfg(not(unix))]
    let _ = shell;
    *FOLDERS.lock().unwrap() = None;
}

/// The PATH as Windows has it written down, which is what a program started from the
/// desktop from now on is given. An app that is already running keeps the PATH it was
/// started with, so a folder an installer has just added is only here.
#[cfg(windows)]
mod windows_path {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    const MACHINE: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";
    const USER: &str = "Environment";
    const MORE_DATA: u32 = 234;

    fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text).encode_wide().chain([0]).collect()
    }

    /// One written-down variable, with the `%NAME%`s Windows can fill in filled in.
    fn read(root: HKEY, key: &str, name: &str) -> Option<String> {
        let (key, name) = (wide(key), wide(name));
        let mut room = 8 * 1024usize;
        for _ in 0..4 {
            let mut text = vec![0u16; room];
            let mut bytes = (room * 2) as u32;
            // Safe: every pointer is to a live buffer of the size said, and the strings end in a zero.
            let said = unsafe { RegGetValueW(root, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), text.as_mut_ptr().cast(), &mut bytes) };
            if said == MORE_DATA {
                room = (bytes as usize / 2 + 1).max(room * 2);
                continue;
            }
            if said != 0 {
                return None;
            }
            let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
            return Some(OsString::from_wide(&text[..end]).to_string_lossy().into_owned()).filter(|value| !value.is_empty());
        }
        None
    }

    /// A variable of the user's, or else of the computer's.
    fn variable(name: &str) -> Option<String> {
        read(HKEY_CURRENT_USER, USER, name).or_else(|| read(HKEY_LOCAL_MACHINE, MACHINE, name))
    }

    /// The computer's PATH and then the user's, as Windows joins them. A `%NAME%` this
    /// app was started without (a variable the same installer has just made) is filled
    /// in from where it is written down.
    pub fn now() -> Vec<OsString> {
        [read(HKEY_LOCAL_MACHINE, MACHINE, "Path"), read(HKEY_CURRENT_USER, USER, "Path")].into_iter().flatten().map(|path| super::fill_in(&path, &variable).into()).collect()
    }
}

/// Text with each `%NAME%` that `value` knows replaced; one it does not know is left as it is.
#[cfg(any(windows, test))]
fn fill_in(text: &str, value: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let named = from[1..].find('%').map(|end| &from[1..1 + end]).filter(|name| !name.is_empty() && !name.contains([';', '\\', '/']));
        match named.and_then(|name| value(name).map(|value| (name, value))) {
            Some((name, value)) => {
                out.push_str(&value);
                rest = &from[name.len() + 2..];
            }
            None => {
                out.push('%');
                rest = &from[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Child programs need the same PATH used to find them (including interpreters).
pub fn child_path() -> Option<std::ffi::OsString> {
    std::env::join_paths(folders().iter()).ok()
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

/// The PATH the user's own shell sets up, asked for once. Two seconds at most.
#[cfg(unix)]
fn login_shell_path() -> Option<std::ffi::OsString> {
    use std::process::{Command, Stdio};
    let shell = std::env::var_os("SHELL").filter(|s| !s.is_empty())?;
    let mut child = Command::new(shell)
        .args(["-ilc", "printf '%s' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < std::time::Duration::from_secs(2) => std::thread::sleep(std::time::Duration::from_millis(40)),
            _ => {
                let _ = child.kill();
                return None;
            }
        }
    }
    let mut text = String::new();
    std::io::Read::read_to_string(child.stdout.as_mut()?, &mut text).ok()?;
    // A shell may greet before it answers; the PATH is the last line.
    let path = text.lines().last()?.trim();
    (!path.is_empty()).then(|| path.into())
}

/// Find a program by the name it is typed with, or take a full path as given.
pub fn find(program: &str) -> Option<Found> {
    find_also(program, &[])
}

/// The same, looking in these folders too when the usual ones do not have it.
pub fn find_also(program: &str, also: &[PathBuf]) -> Option<Found> {
    let asked = Path::new(program);
    let file = if asked.components().count() > 1 {
        asked.is_file().then(|| asked.to_path_buf())?
    } else {
        folders().iter().chain(also).find_map(|dir| in_folder(dir, program))?
    };
    Some(as_started(&file))
}

#[cfg(windows)]
fn in_folder(dir: &Path, name: &str) -> Option<PathBuf> {
    // A name that already says what kind of file it is, is looked for as written.
    if Path::new(name).extension().is_some() {
        let file = dir.join(name);
        return file.is_file().then_some(file);
    }
    // A real program first; the scripts npm leaves beside it after.
    [".exe", ".cmd", ".bat"].iter().map(|ext| dir.join(format!("{name}{ext}"))).find(|file| file.is_file())
}

#[cfg(unix)]
fn in_folder(dir: &Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let file = dir.join(name);
    let runs = std::fs::metadata(&file).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false);
    runs.then_some(file)
}

/// How a found file is started. On Windows, npm installs a program as a small
/// `.cmd` script that calls the real one: `node` with a script, or, for a package
/// that ships a program of its own (Claude Code does), that program. Starting the
/// real one directly keeps `cmd.exe`, and its reading of `&` and `%`, out of the way,
/// and makes the process the office started the program itself, not a shell around it.
fn as_started(file: &Path) -> Found {
    let plain = Found { program: file.to_string_lossy().into_owned(), before: Vec::new(), through_shell: false };
    let is_script = file.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    if !is_script {
        return plain;
    }
    let dir = file.parent().unwrap_or(Path::new("."));
    let script = std::fs::read_to_string(file).ok().and_then(|text| npm_script(&text)).map(|rest| dir.join(rest));
    match script.filter(|s| s.is_file()) {
        Some(program) if program.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) => Found { program: program.to_string_lossy().into_owned(), ..plain },
        Some(script) => {
            let beside = dir.join("node.exe");
            let node = if beside.is_file() { Some(beside) } else { real_node() };
            match node {
                Some(node) => Found { program: node.to_string_lossy().into_owned(), before: vec![script.to_string_lossy().into_owned()], through_shell: false },
                // No Node to start it with directly: the script itself knows how, through cmd.exe.
                None => Found { through_shell: true, ..plain },
            }
        }
        None => Found { through_shell: true, ..plain },
    }
}

/// Node itself: the program, never a script of the same name. A global npm package
/// called `node` leaves a `node.cmd` that can come first on the PATH and runs nothing.
fn real_node() -> Option<PathBuf> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    folders().iter().map(|dir| dir.join(name)).find(|file| file.is_file())
}

/// What an npm `.cmd` file runs, as a path from the folder the file is in: a script
/// for `node`, or a program of the package's own.
fn npm_script(text: &str) -> Option<String> {
    // The last line reads: "%_prog%"  "%dp0%\node_modules\pkg\bin\cli.js" %*
    // or, with no `node` in it:    "%dp0%\node_modules\pkg\bin\tool.exe"   %*
    let marker = "\"%dp0%\\";
    let from = text.rfind(marker)? + marker.len();
    let rest = &text[from..];
    let to = rest.find('"')?;
    let path = &rest[..to];
    [".js", ".mjs", ".cjs", ".exe"].iter().any(|kind| path.to_lowercase().ends_with(kind)).then(|| path.replace('\\', "/"))
}

/// Whether a word can be handed to `cmd.exe` without it meaning more than it says.
pub fn plain_word(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | ':' | '/' | '\\' | '=' | ',' | '{' | '}' | '@'))
}

/// The program and every word for it, ready to start. Refuses when the only way
/// in is through a shell and a word could be read as a command.
pub fn command_line(found: &Found, args: &[String]) -> Result<(String, Vec<String>), String> {
    if found.through_shell {
        if let Some(word) = args.iter().find(|w| !plain_word(w)) {
            return Err(format!("This program can only be started through cmd.exe, which would misread \u{201c}{}\u{201d}. Start it without a task and type the task in its terminal.", shorten(word, 40)));
        }
        let mut all = vec!["/d".to_string(), "/c".to_string(), found.program.clone()];
        all.extend(args.iter().cloned());
        return Ok(("cmd.exe".into(), all));
    }
    let mut all = found.before.clone();
    all.extend(args.iter().cloned());
    Ok((found.program.clone(), all))
}

/// What `--version` prints, shortened to its first line. Empty when it does not answer in time.
pub fn version(found: &Found) -> String {
    version_from(&ask(found, &["--version".to_string()], 8))
}

/// What a program prints when it is asked something, with no window and no input.
/// Empty when it does not answer within `secs`.
fn ask(found: &Found, words: &[String], secs: u64) -> String {
    answer(found, words, secs).map(|said| said.out).unwrap_or_default()
}

/// What a program said when it was asked something: whether it ended well, and what
/// it printed, on each of the two ways it has of printing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    pub ok: bool,
    pub out: String,
    pub err: String,
}

/// Ask a program something, with no window and no input, as one of the office's own
/// desks would start it. None when it could not be started, or did not answer within
/// `secs`. Only for answers of a few lines: they are read once it has ended.
pub fn answer(found: &Found, words: &[String], secs: u64) -> Option<Answer> {
    use std::process::{Command, Stdio};
    let (program, args) = command_line(found, words).ok()?;
    let mut command = Command::new(program);
    if let Some(path) = child_path() {
        command.env("PATH", path);
    }
    // Asked as a desk's program would be: not as a session inside whatever started this app.
    for name in crate::pty::borrowed_names() {
        command.env_remove(name);
    }
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window may flash up for a question nobody asked to see answered.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().ok()?;
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < std::time::Duration::from_secs(secs) => std::thread::sleep(std::time::Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let mut said = Answer { ok: status.success(), ..Answer::default() };
    if let Some(out) = child.stdout.as_mut() {
        let _ = std::io::Read::read_to_string(out, &mut said.out);
    }
    if let Some(err) = child.stderr.as_mut() {
        let _ = std::io::Read::read_to_string(err, &mut said.err);
    }
    Some(said)
}

// ── who is signed in ───────────────────────────────────────────────────────

/// Whether someone is signed in to a program, as far as the program says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// None when it has not said, or what it said was not understood.
    pub signed_in: Option<bool>,
    /// A few words to show: who is signed in, or how, or that nobody is. Empty when not known.
    pub label: String,
}

/// Read what a program answered when asked who is signed in.
///
/// Claude Code answers in JSON with `loggedIn`; of the rest only the address and the
/// plan are taken, never the answer whole, which names folders and an organisation.
/// Codex answers with a line of its own ("Logged in using ChatGPT", printed as an
/// error would be), and ends badly when nobody is signed in. An answer that says
/// neither is not taken for "signed out": a program that could not read its own
/// settings has not said that.
pub fn account_from(said: &Answer) -> Account {
    let signed_out = || Account { signed_in: Some(false), label: "Signed out".into() };
    if let Ok(serde_json::Value::Object(fields)) = serde_json::from_str::<serde_json::Value>(said.out.trim()) {
        return match fields.get("loggedIn").and_then(serde_json::Value::as_bool) {
            Some(true) => {
                let text = |name: &str| fields.get(name).and_then(serde_json::Value::as_str).map(clean).filter(|s| !s.is_empty());
                let plan = text("subscriptionType").map(|plan| {
                    let mut letters = plan.chars();
                    letters.next().map(|first| first.to_uppercase().chain(letters).collect::<String>()).unwrap_or_default()
                });
                let label = match (text("email"), plan) {
                    (Some(who), Some(plan)) => format!("Signed in as {who} · {plan}"),
                    (Some(who), None) => format!("Signed in as {who}"),
                    (None, Some(plan)) => format!("Signed in · {plan}"),
                    (None, None) => "Signed in".into(),
                };
                Account { signed_in: Some(true), label }
            }
            Some(false) => signed_out(),
            None => Account::default(),
        };
    }
    let line = said.out.lines().chain(said.err.lines()).map(clean).find(|l| !l.is_empty()).unwrap_or_default();
    let lower = line.to_lowercase();
    if ["not logged in", "not signed in", "logged out", "signed out", "no credentials"].iter().any(|says| lower.contains(says)) {
        return signed_out();
    }
    if !said.ok || line.is_empty() {
        return Account::default();
    }
    // Its own line, without what follows a dash: Codex shows part of an API key there.
    let label = line.split(" - ").next().unwrap_or(&line).trim();
    Account { signed_in: Some(true), label: shorten(label, 80) }
}

/// One line of what a program printed, without the codes that colour it or move the cursor.
fn clean(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // An escape sequence: up to the letter that ends it.
            if chars.peek() == Some(&'[') {
                chars.next();
                for end in chars.by_ref() {
                    if end.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if !c.is_control() {
            plain.push(c);
        }
    }
    shorten(&plain, 120)
}

/// "2.1.290 (Claude Code)" and "codex-cli 0.160.0" both become their number.
fn version_from(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let number = line.split_whitespace().map(|w| w.trim_start_matches('v')).find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && w.contains('.'));
    shorten(number.unwrap_or(line), 24)
}

/// First part of a text on one line, cut on a character boundary.
pub fn shorten(text: &str, max_chars: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        return flat;
    }
    let mut cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
    cut.push('\u{2026}');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(id: &str) -> Harness {
        built_in().into_iter().find(|h| h.id == id).unwrap()
    }

    #[test]
    fn claude_is_started_with_an_id_a_name_and_the_task_last() {
        let args = start_args(&known("claude"), Some("11111111-2222-4333-8444-555555555555"), "Fix the total", "Fix the checkout total & test it", true);
        assert_eq!(
            args,
            ["--session-id", "11111111-2222-4333-8444-555555555555", "--name", "Fix the total", "--worktree", "Fix the checkout total & test it"]
        );
        // No task: nothing is invented to stand in for one.
        assert_eq!(start_args(&known("claude"), Some("id"), "", "  ", false), ["--session-id", "id"]);
    }

    #[test]
    fn each_program_takes_its_task_its_own_way() {
        assert_eq!(start_args(&known("codex"), None, "Ignored", "Type the API", false), ["-c", "tui.notifications=true", "-c", "tui.terminal_title=[\"session-id\",\"thread-title\"]", "Type the API"]);
        assert_eq!(start_args(&known("antigravity"), None, "", "Write docs", false), ["--prompt-interactive", "Write docs"]);
        assert_eq!(start_args(&known("gemini"), None, "", "Write docs", false), ["--prompt-interactive", "Write docs"]);
        // A program known only by name is never handed words it may not understand.
        assert!(start_args(&known("opencode"), Some("x"), "A name", "A task", true).is_empty());
    }

    #[test]
    fn carrying_on_needs_whatever_the_program_needs() {
        assert_eq!(resume_args(&known("claude"), Some("abc")).unwrap(), ["--resume", "abc"]);
        assert_eq!(resume_args(&known("claude"), None), None);
        assert_eq!(resume_args(&known("codex"), Some("abc")).unwrap(), ["-c", "tui.notifications=true", "-c", "tui.terminal_title=[\"session-id\",\"thread-title\"]", "resume", "abc"]);
        // Hermes carries on its latest conversation without being told which.
        assert_eq!(resume_args(&known("hermes"), None).unwrap(), ["--continue"]);
        // So does Antigravity CLI, which keeps its conversations by folder.
        assert_eq!(resume_args(&known("antigravity"), None).unwrap(), ["--continue"]);
        assert_eq!(resume_args(&known("gemini"), Some("abc")), None);
        assert!(can_resume(&known("hermes"), None) && !can_resume(&known("opencode"), Some("abc")));
        assert_eq!(resume_scope(&known("claude"), Some("abc")), ResumeScope::Desk);
        assert_eq!(resume_scope(&known("codex"), None), ResumeScope::None);
        assert_eq!(resume_scope(&known("antigravity"), None), ResumeScope::Folder);
        assert_eq!(resume_scope(&known("hermes"), None), ResumeScope::Latest);
        assert_eq!(resume_scope(&known("gemini"), Some("abc")), ResumeScope::None);
    }

    #[test]
    fn the_script_behind_an_npm_command_is_found() {
        let codex = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n)\r\n\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n";
        assert_eq!(npm_script(codex).unwrap(), "node_modules/@openai/codex/bin/codex.js");
        assert_eq!(npm_script("@echo off\r\npython \"%~dp0\\tool.py\" %*\r\n"), None);
        // A package that ships a program of its own, as Claude Code's does: no `node` in it.
        let claude = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\"%dp0%\\node_modules\\@anthropic-ai\\claude-code\\bin\\claude.exe\"   %*\r\n";
        assert_eq!(npm_script(claude).unwrap(), "node_modules/@anthropic-ai/claude-code/bin/claude.exe");
    }

    #[test]
    fn a_name_between_percent_signs_is_filled_in_from_where_it_is_written_down() {
        // As nvm for Windows writes itself into the PATH, with variables it has only just made.
        let known = |name: &str| match name {
            "NVM_HOME" => Some("C:\\nvm".to_string()),
            "NVM_SYMLINK" => Some("C:\\nvm4w\\nodejs".to_string()),
            _ => None,
        };
        assert_eq!(fill_in("C:\\Windows;%NVM_HOME%;%NVM_SYMLINK%;%NOT_SET%\\bin;100%", &known), "C:\\Windows;C:\\nvm;C:\\nvm4w\\nodejs;%NOT_SET%\\bin;100%");
        assert_eq!(fill_in("%%NVM_HOME%", &known), "%C:\\nvm");
        assert_eq!(fill_in("", &known), "");
    }

    #[cfg(windows)]
    #[test]
    fn the_path_windows_has_written_down_is_read_with_its_names_filled_in() {
        // The computer's own PATH is written with %SystemRoot% in it.
        let all = windows_path::now().iter().map(|path| path.to_string_lossy().to_lowercase()).collect::<Vec<_>>().join(";");
        assert!(all.contains("system32"), "{all}");
        assert!(!all.contains("%systemroot%"), "{all}");
    }

    #[test]
    fn the_folders_are_listed_afresh_once_asked_to_look_again() {
        let first = folders();
        assert!(!first.is_empty());
        look_again(false);
        let again = folders();
        assert!(!Arc::ptr_eq(&first, &again));
        // Where Node.js is put by its installer is looked in whatever the PATH says.
        #[cfg(windows)]
        assert!(again.iter().any(|dir| dir.ends_with("nodejs")), "{again:?}");
    }

    #[test]
    fn a_script_npm_wrote_over_a_program_of_its_own_is_started_as_that_program() {
        let dir = std::env::temp_dir().join(format!("moshpit-shim-exe-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("pkg")).unwrap();
        std::fs::write(dir.join("pkg").join("tool.exe"), "not really a program").unwrap();
        let shim = dir.join(if cfg!(windows) { "tool.cmd" } else { "tool" });
        std::fs::write(&shim, "@ECHO off\r\n\"%dp0%\\pkg\\tool.exe\"   %*\r\n").unwrap();
        let found = find(&shim.to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
        if cfg!(windows) {
            // Itself: no shell around it to misread a task, and its own process to be asked about.
            let found = found.unwrap();
            assert_eq!((found.program.replace('/', "\\").to_lowercase(), found.before.len(), found.through_shell), (dir.join("pkg").join("tool.exe").to_string_lossy().to_lowercase(), 0, false));
            assert_eq!(command_line(&found, &["fix it & calc".into()]).unwrap().1, ["fix it & calc"]);
        }
    }

    #[test]
    fn a_script_written_the_way_npm_writes_one_is_started_with_node() {
        let dir = std::env::temp_dir().join(format!("moshpit-shim-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tool.mjs"), "console.log(1)").unwrap();
        let shim = dir.join(if cfg!(windows) { "tool.cmd" } else { "tool" });
        std::fs::write(&shim, r#"@ECHO off
"%_prog%"  "%dp0%\tool.mjs" %*
"#,
        )
        .unwrap();
        let found = find(&shim.to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
        if cfg!(windows) {
            let found = found.unwrap();
            assert!(!found.through_shell, "{found:?}");
            assert_eq!(found.before, [dir.join("tool.mjs").to_string_lossy().into_owned()]);
            // Started with node.exe itself, not a script that happens to be called node.
            assert!(found.program.to_lowercase().ends_with("node.exe"), "{found:?}");
        }
    }

    #[test]
    fn a_shell_is_only_ever_given_plain_words() {
        let shell = Found { program: "C:\\tools\\thing.cmd".into(), before: vec![], through_shell: true };
        let (program, args) = command_line(&shell, &["--resume".into(), "abc-123".into()]).unwrap();
        assert_eq!(program, "cmd.exe");
        assert_eq!(args, ["/d", "/c", "C:\\tools\\thing.cmd", "--resume", "abc-123"]);
        for bad in ["fix it & calc", "a | b", "100%", "say \"hi\"", "a > b", "$(id)", "a\nb"] {
            assert!(command_line(&shell, &[bad.to_string()]).is_err(), "{bad:?}");
        }
        // Started directly, any text is only ever one word to the program.
        let direct = Found { program: "node".into(), before: vec!["cli.js".into()], through_shell: false };
        assert_eq!(command_line(&direct, &["fix it & calc".into()]).unwrap(), ("node".to_string(), vec!["cli.js".to_string(), "fix it & calc".to_string()]));
    }

    #[test]
    fn a_program_is_installed_by_npm_and_updated_its_own_way_first() {
        assert_eq!(install_runner(&known("codex")), Some(Runner::Npm(words(&["install", "-g", "@openai/codex"]))));
        assert_eq!(update_runner(&known("codex")), Some(Runner::Itself(words(&["update"]))));
        assert_eq!(update_runner(&known("gemini")), Some(Runner::Npm(words(&["install", "-g", "@google/gemini-cli@latest"]))));
        // Hermes updates itself but is not installed from npm; Cursor is neither.
        assert_eq!((install_runner(&known("hermes")), update_runner(&known("hermes")).is_some()), (None, true));
        // Antigravity CLI is the same as Hermes in this: `agy update`, and no package to install it from.
        assert_eq!((install_runner(&known("antigravity")), update_runner(&known("antigravity"))), (None, Some(Runner::Itself(words(&["update"])))));
        assert_eq!(known("antigravity").program, "agy");
        assert_eq!((install_runner(&known("cursor")), update_runner(&known("cursor"))), (None, None));
        // A scoped name is a plain word, so it can go through cmd.exe.
        assert!(plain_word("@openai/codex@latest"));
        // What is shown is what runs.
        assert_eq!(install_runner(&known("codex")).unwrap().line("codex"), "npm install -g @openai/codex");
        assert_eq!(update_runner(&known("claude")).unwrap().line("claude"), "claude update");
        let mine = Harness { install: words(&["pipx", "install", "my agent"]), ..row("mine", "Mine", "Mine", "mine") };
        assert_eq!(install_runner(&mine).unwrap().line("mine"), "pipx install \"my agent\"");
    }

    #[test]
    fn a_program_signs_in_and_says_who_is_signed_in_with_its_own_commands() {
        assert_eq!(login_runner(&known("claude")).unwrap().line("claude"), "claude auth login");
        assert_eq!(login_runner(&known("codex")).unwrap().line("codex"), "codex login");
        assert_eq!(known("claude").account, words(&["auth", "status", "--json"]));
        assert_eq!(known("codex").account, words(&["login", "status"]));
        // The rest have no such commands that the office knows: signing in is done in their terminals.
        assert!(built_in().iter().filter(|h| h.id != "claude" && h.id != "codex").all(|h| login_runner(h).is_none() && h.account.is_empty()));
        // A row that names a program which runs another: the other comes before the command, its flags do not.
        let wrapped = Harness { args: words(&["cli.mjs", "--quiet", "extra"]), ..row("mine", "Mine", "Mine", "node") };
        assert_eq!(own_words(&wrapped, &words(&["auth", "status"])), words(&["cli.mjs", "auth", "status"]));
        assert_eq!(own_words(&known("codex"), &words(&["login", "status"])), words(&["login", "status"]));
    }

    #[test]
    fn who_is_signed_in_is_read_from_what_each_program_says() {
        let said = |ok: bool, out: &str, err: &str| account_from(&Answer { ok, out: out.into(), err: err.into() });
        let known = |signed_in: bool, label: &str| Account { signed_in: Some(signed_in), label: label.into() };
        // Claude Code, as it prints it: over several lines, with more than is shown.
        let claude = "{\n  \"loggedIn\": true,\n  \"authMethod\": \"claude.ai\",\n  \"apiProvider\": \"firstParty\",\n  \"configDirectory\": \"C:\\\\Users\\\\Someone\\\\.claude\",\n  \"email\": \"someone@example.com\",\n  \"orgId\": \"org-1\",\n  \"orgName\": \"Someone's Organisation\",\n  \"subscriptionType\": \"max\"\n}\n";
        let account = said(true, claude, "");
        assert_eq!(account, known(true, "Signed in as someone@example.com · Max"));
        assert!(!account.label.contains("Organisation") && !account.label.contains("Users"));
        assert_eq!(said(true, "{\"loggedIn\":true,\"authMethod\":\"api_key\"}", ""), known(true, "Signed in"));
        assert_eq!(said(true, "{\"loggedIn\":true,\"subscriptionType\":\"pro\"}", ""), known(true, "Signed in · Pro"));
        // Signed out, whether or not it also ends badly.
        assert_eq!(said(false, "{\"loggedIn\": false, \"authMethod\": \"none\"}", ""), known(false, "Signed out"));
        assert_eq!(said(true, "{\"loggedIn\": false}", ""), known(false, "Signed out"));
        // Codex prints its line as an error would be printed, and ends well.
        assert_eq!(said(true, "", "Logged in using ChatGPT\n"), known(true, "Logged in using ChatGPT"));
        assert_eq!(said(true, "", "Logged in using an API key - sk-proj-***ABCD\n"), known(true, "Logged in using an API key"));
        assert_eq!(said(false, "", "Not logged in\n"), known(false, "Signed out"));
        assert_eq!(said(true, "\u{1b}[31mNot logged in\u{1b}[0m\n", ""), known(false, "Signed out"));
        // Anything else is not known, and never taken for signed out.
        for (ok, out, err) in [(false, "", "error: could not read config.toml"), (false, "", ""), (true, "", ""), (true, "{\"status\":\"ok\"}", ""), (false, "{ not json", "")] {
            assert_eq!(said(ok, out, err), Account::default(), "{ok} {out:?} {err:?}");
        }
        // A long line is cut, and what colours it is not shown.
        let long = said(true, &format!("\u{1b}[1mLogged in {}\u{1b}[0m", "x".repeat(300)), "");
        assert!(long.signed_in == Some(true) && long.label.chars().count() <= 80 && !long.label.contains('\u{1b}'));
    }

    #[test]
    fn a_program_asked_something_says_how_it_ended_and_on_which_side_it_printed() {
        let shell = if cfg!(windows) { Found { program: "cmd.exe".into(), before: words(&["/d", "/c"]), through_shell: false } } else { Found { program: "/bin/sh".into(), before: words(&["-c"]), through_shell: false } };
        let then = if cfg!(windows) { "&" } else { ";" };
        let both = answer(&shell, &[format!("echo out-word{then} echo err-word 1>&2{then} exit 3")], 20).unwrap();
        assert!(!both.ok && both.out.contains("out-word") && both.err.contains("err-word"), "{both:?}");
        assert!(answer(&shell, &words(&["echo fine"]), 20).unwrap().ok);
        assert_eq!(answer(&Found { program: "no-such-program-moshpit".into(), before: vec![], through_shell: false }, &[], 5), None);
    }

    #[test]
    fn only_a_package_name_is_taken_for_one() {
        for good in ["opencode-ai", "@openai/codex", "@qwen-code/qwen-code", "a.b_c"] {
            assert!(package_name(good), "{good}");
        }
        for bad in ["", "@scope", "@/x", "Upper", "a b", "a&b", "../x", ".hidden", "@s/.x", "x;y"] {
            assert!(!package_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_newer_version_is_told_apart() {
        assert!(newer("0.160.0", "0.161.0"));
        assert!(newer("2.1.290", "2.1.294"));
        assert!(newer("0.2.1", "0.63.0"));
        assert!(newer("1.2", "1.2.1"));
        assert!(!newer("2.1.294", "2.1.294"));
        assert!(!newer("0.161.0", "0.160.0"));
        assert!(!newer("v1.0.0", "1.0.0"));
        assert!(!newer("1.0.0-beta.3", "1.0.0"));
        // Something that is not a version is never taken for an older one.
        assert!(!newer("", "1.0.0") && !newer("Hermes Agent", "1.0.0") && !newer("1.0.0", ""));
    }

    #[test]
    fn a_version_is_its_number() {
        assert_eq!(version_from("2.1.290 (Claude Code)\n"), "2.1.290");
        assert_eq!(version_from("codex-cli 0.160.0"), "0.160.0");
        assert_eq!(version_from("\n0.2.1\n"), "0.2.1");
        assert_eq!(version_from("Hermes Agent v0.21.2"), "0.21.2");
        assert_eq!(version_from("some words"), "some words");
        assert_eq!(version_from(""), "");
    }

    #[test]
    fn a_row_written_by_hand_replaces_or_joins_the_table() {
        let dir = std::env::temp_dir().join(format!("moshpit-harness-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("harnesses.json");
        std::fs::write(
            &file,
            r#"[
              {"id":"codex","name":"Codex","tag":"Codex","program":"codex-nightly","task":"last"},
              {"id":"mine","name":"My Agent","tag":"Mine","program":"my-agent","task":{"flag":"--ask"},"resume":["--again"]},
              {"id":"bad","name":"Bad","tag":"Bad","program":"calc & del"},
              {"id":"sly","name":"Sly","tag":"Sly","program":"sly","package":"x & calc"}
            ]"#,
        )
        .unwrap();
        let (all, problems) = table_checked(&file);
        assert_eq!(all.iter().find(|h| h.id == "codex").unwrap().program, "codex-nightly");
        let mine = all.iter().find(|h| h.id == "mine").unwrap();
        assert_eq!(start_args(mine, None, "", "Do it", false), ["--ask", "Do it"]);
        assert!(all.iter().all(|h| h.id != "bad" && h.id != "sly"));
        assert_eq!(all.len(), built_in().len() + 1);
        // The rows that were skipped are named, so whoever wrote them can see why, and where.
        assert_eq!(problems.len(), 2);
        assert!(problems[0].text.contains("\"bad\"") && problems[1].text.contains("\"sly\""), "{problems:?}");
        assert!(problems.iter().all(|p| p.file == file.to_string_lossy() && p.text.contains(&*file.to_string_lossy())));

        // A file that is not JSON is said to be so, and the built-in table stands.
        std::fs::write(&file, r#"[{"id":"x","program":"C:\Tools\x.exe"}]"#).unwrap();
        let (all, problems) = table_checked(&file);
        assert_eq!(all.len(), built_in().len());
        assert!(problems.len() == 1 && problems[0].text.contains("could not be read"), "{problems:?}");
        // A single backslash is the usual slip, and is explained; with the line to open the file at.
        assert!(problems[0].text.contains("is written \\\\") && problems[0].line == Some(1), "{problems:?}");
        // A missing comma is not a backslash.
        std::fs::write(&file, "[\n  {\"id\":\"x\" \"name\":\"X\"}\n]").unwrap();
        let (_, problems) = table_checked(&file);
        assert!(!problems[0].text.contains("is written") && problems[0].line == Some(2), "{problems:?}");
        // No file at all is no problem.
        let _ = std::fs::remove_dir_all(&dir);
        assert!(table_checked(&file).1.is_empty());
    }
}
