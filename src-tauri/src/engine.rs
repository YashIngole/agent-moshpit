//! The engine: it owns the desks and the terminals, starts programs, looks at
//! them twice a second, and tells the window when something changed.
//!
//! Everything it does is quick and done under one lock, so there is no queue of
//! messages to follow: a command from the window changes the state and publishes it.

use crate::harness::{self, Found, Harness, Runner, SessionFrom, StatusFrom};
use crate::model::{now_ms, HarnessView, JobView, NewAgent, Phase, Snapshot};
use crate::office::{title_from, Office, SavedDesk, Transition};
use crate::pty::{Launch, Sink, Terminals};
use crate::status::{self, Signal};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// What the engine needs from the desktop around it. Tests supply a quiet one.
pub trait Shell: Send + Sync + 'static {
    fn snapshot(&self, snapshot: &Snapshot);
    /// Say something in the notification centre about a desk; a click on it opens that desk.
    fn notify(&self, title: &str, body: &str, desk: &str);
    fn save(&self, revision: u64, desks: &[SavedDesk]);
    /// What each desk's terminal last showed, kept for the next run. Tests keep nothing.
    fn save_screens(&self, _screens: &[(String, Vec<u8>)]) {}
}

/// How often running programs are looked at.
const LOOK_EVERY: Duration = Duration::from_millis(500);
/// How many looks pass between checks of slower things: the branch, Codex's files.
const SLOW_EVERY: u64 = 6;
/// How much of each desk's terminal is kept on disk for the next run: its screen as drawn,
/// and this many lines above it.
const SCREEN_HISTORY: usize = 500;
/// How often npm is asked whether a program has a newer version.
const UPDATES_EVERY: Duration = Duration::from_secs(12 * 60 * 60);

/// Install a program, bring it up to date, or sign someone in to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Install,
    Update,
    SignIn,
}

impl JobKind {
    /// What it is called in a terminal's id, and to the window.
    fn word(self) -> &'static str {
        match self {
            JobKind::Install => "install",
            JobKind::Update => "update",
            JobKind::SignIn => "signin",
        }
    }

    /// What it is called in a sentence.
    fn name(self) -> &'static str {
        match self {
            JobKind::Install => "install",
            JobKind::Update => "update",
            JobKind::SignIn => "sign-in",
        }
    }
}

/// How often a program believed signed out, with a desk of its own or its sign-in running,
/// is asked again: every this many looks. Signing in happens inside those, and ends nothing.
const ACCOUNT_EVERY: u64 = 10;

/// An install, an update or a sign-in, and how it went.
#[derive(Debug, Clone)]
struct Job {
    id: String,
    harness: String,
    kind: JobKind,
    running: bool,
    ok: Option<bool>,
}

/// What is known about one program of the table on this computer.
#[derive(Debug, Clone, Default)]
struct Installed {
    found: Option<Found>,
    version: String,
    /// Who is signed in to it, as it last said. See `Handle::wonder`.
    account: harness::Account,
}

/// A desk whose program is not in the table this time: a row taken out, or a
/// `harnesses.json` that could not be read. It cannot be shown, but it is kept as it
/// was found and saved with the others, so it is back when the table has its program again.
struct Aside {
    /// Where it was in the list of desks, to be put back there.
    place: usize,
    desk: SavedDesk,
    /// What its terminal last showed, if that was kept.
    screen: Option<Vec<u8>>,
}

struct State {
    office: Office,
    /// Selected terminals being read, for completion flags and notifications.
    watched: HashSet<String>,
    /// Uncovered terminal panes, including the recording's original pane after a focus change.
    visible: HashSet<String>,
    /// Whether the window is in front. A terminal behind another app is not being watched.
    focused: bool,
    installed: HashMap<String, Installed>,
    /// The newest version of each program that npm has been asked about.
    latest: HashMap<String, String>,
    jobs: Vec<Job>,
    /// What the window was last told, to send nothing when nothing changed.
    told: Option<(Vec<crate::model::AgentView>, Vec<HarnessView>, Vec<JobView>)>,
    /// The desks as last saved, those held aside among them, to save only what changed.
    saved: Vec<SavedDesk>,
    revision: u64,
    looks: u64,
    /// Voice text must not end up in an app-owned screen file through PTY echo.
    voice_used: HashSet<String>,
}

/// Something to tell the desktop. Queued, and said by a thread that holds no lock.
enum Out {
    Snapshot(Snapshot),
    Save(u64, Vec<SavedDesk>),
    /// A title, what it says, and the desk it is about.
    Notify(String, String, String),
}

struct Inner {
    shell: Arc<dyn Shell>,
    /// What is waiting to be told to the desktop. See `Out`.
    out: Mutex<Sender<Out>>,
    table: Vec<Harness>,
    /// The desks that table has no program for. See `Aside`.
    aside: Vec<Aside>,
    terms: Arc<Terminals>,
    state: Mutex<State>,
    home: PathBuf,
    stopping: AtomicBool,
    mcp: Mutex<Option<Arc<crate::mcp::Hub>>>,
    catalogs: crate::catalog::Cache,
    /// The programs waiting to be asked who is signed in to them, and the way to add one.
    /// One thread asks them, one at a time, and none waits twice.
    asking: Mutex<HashSet<String>>,
    accounts: Mutex<Sender<String>>,
}

/// The engine, as the rest of the app holds it. Cheap to clone.
#[derive(Clone)]
pub struct Handle {
    inner: Arc<Inner>,
}

/// Start the engine: everyone from the last run is back at a desk, asleep, with
/// what their terminal last showed (`screens`) on it until their program starts again.
pub fn start(shell: Arc<dyn Shell>, table: Vec<Harness>, saved: Vec<SavedDesk>, screens: Vec<(String, Vec<u8>)>) -> Handle {
    // The terminals report a program's end to the engine, which is made after them.
    let slot: Arc<Mutex<Option<Handle>>> = Arc::new(Mutex::new(None));
    let for_exit = slot.clone();
    let terms = Terminals::new(Arc::new(move |id: &str, ok: bool| {
        let handle = for_exit.lock().unwrap().clone();
        if let Some(handle) = handle {
            handle.exited(id, ok);
        }
    }));
    let known: HashSet<&str> = table.iter().map(|h| h.id.as_str()).collect();
    let mut seated: Vec<SavedDesk> = Vec::new();
    let mut aside: Vec<Aside> = Vec::new();
    for (place, desk) in saved.iter().cloned().enumerate() {
        if known.contains(desk.harness.as_str()) {
            seated.push(desk);
        } else {
            aside.push(Aside { place, desk, screen: None });
        }
    }
    for (id, printed) in screens {
        if seated.iter().any(|d| d.id == id) {
            terms.restore(&id, &printed);
        } else if let Some(kept) = aside.iter_mut().find(|a| a.desk.id == id) {
            kept.screen = Some(printed);
        }
    }
    let state = State {
        office: Office::restore(seated, now_ms()),
        watched: HashSet::new(),
        visible: HashSet::new(),
        focused: false,
        installed: HashMap::new(),
        latest: HashMap::new(),
        jobs: Vec::new(),
        told: None,
        saved,
        revision: 0,
        looks: 0,
        voice_used: HashSet::new(),
    };
    let home = harness::home_dir().unwrap_or_default();
    // The desktop is told things by this thread alone. Showing a snapshot or a
    // notice waits for the app's main thread, which may itself be waiting for the
    // engine's lock: so nothing is ever said to the desktop while that lock is held.
    let (out, queued) = mpsc::channel::<Out>();
    let teller = shell.clone();
    std::thread::spawn(move || {
        for message in queued {
            match message {
                Out::Snapshot(snapshot) => teller.snapshot(&snapshot),
                Out::Save(revision, desks) => teller.save(revision, &desks),
                Out::Notify(title, body, desk) => teller.notify(&title, &body, &desk),
            }
        }
    });
    let (accounts, wondered) = mpsc::channel::<String>();
    let handle = Handle {
        inner: Arc::new(Inner {
            shell,
            out: Mutex::new(out),
            table,
            aside,
            terms,
            state: Mutex::new(state),
            home,
            stopping: AtomicBool::new(false),
            mcp: Mutex::new(None),
            catalogs: crate::catalog::Cache::default(),
            asking: Mutex::new(HashSet::new()),
            accounts: Mutex::new(accounts),
        }),
    };
    *slot.lock().unwrap() = Some(handle.clone());
    handle.refresh_places();

    // Asking a program who is signed in starts that program and waits for it: done by
    // this thread alone, so never under the engine's lock and never two at once.
    let asker = handle.clone();
    std::thread::spawn(move || {
        for id in wondered {
            if asker.inner.stopping.load(Ordering::Relaxed) {
                break;
            }
            asker.ask_account(&id);
            asker.inner.asking.lock().unwrap().remove(&id);
        }
    });

    let looker = handle.clone();
    std::thread::spawn(move || {
        while !looker.inner.stopping.load(Ordering::Relaxed) {
            std::thread::sleep(LOOK_EVERY);
            if looker.inner.stopping.load(Ordering::Relaxed) {
                break;
            }
            looker.look();
        }
    });
    // Finding the programs and asking each its version takes a moment; the window does not
    // wait for it. Then, unless told not to, npm is asked now and then for the newest versions.
    let finder = handle.clone();
    std::thread::spawn(move || {
        finder.find_programs();
        if std::env::var_os("MOSHPIT_NO_UPDATE_CHECK").is_some() {
            return;
        }
        loop {
            finder.check_updates();
            let next = std::time::Instant::now() + UPDATES_EVERY;
            while std::time::Instant::now() < next {
                if finder.inner.stopping.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    });
    handle
}

impl Handle {
    pub(crate) fn enable_mcp(&self, path: PathBuf) -> Result<(), String> {
        let mut slot = self.inner.mcp.lock().unwrap();
        if slot.is_none() { *slot = Some(crate::mcp::Hub::start(self.clone(), path)?); }
        Ok(())
    }

    fn mcp_launch(&self, harness: &Harness, id: &str, args: Vec<String>, options: &crate::launch::Options) -> Result<crate::mcp::PreparedLaunch, String> {
        match self.inner.mcp.lock().unwrap().as_ref() {
            Some(hub) => hub.prepare(&harness.id, id, args, crate::launch::prefix_len(harness, options)?),
            None => Ok((args, vec![])),
        }
    }

    fn mcp_revoke(&self, id: &str) {
        if let Some(hub) = self.inner.mcp.lock().unwrap().as_ref() { hub.revoke(id); }
    }

    pub(crate) fn title_locked(&self, id: &str) -> bool {
        self.inner.state.lock().unwrap().office.get(id).is_none_or(|d| d.saved.title_locked)
    }

    pub(crate) fn agent_directory(&self, id: &str, directory: &Path) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        if state.office.get(id).is_none() { return Err("Your desk is gone.".into()); }
        if state.office.agent_directory(id, directory.to_string_lossy().into_owned()) {
            self.place(&mut state, id);
            self.publish(&mut state);
        }
        Ok(())
    }

    pub(crate) fn agent_rename(&self, id: &str, title: &str) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        let desk = state.office.get(id).ok_or("Your desk is gone.")?;
        if desk.saved.title_locked { return Err("The user named this session. Their name is preserved.".into()); }
        state.office.agent_title(id, title);
        self.publish(&mut state);
        Ok(())
    }

    pub(crate) fn agent_activity(&self, id: &str, activity: &str) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        if state.office.get(id).is_none() { return Err("Your desk is gone.".into()); }
        state.office.agent_activity(id, activity);
        self.publish(&mut state);
        Ok(())
    }

    pub(crate) fn agent_attention(&self, id: &str, message: &str) -> Result<(), String> {
        let state = self.inner.state.lock().unwrap();
        let desk = state.office.get(id).ok_or("Your desk is gone.")?;
        self.say(Out::Notify(format!("{} needs you", desk.saved.title), message.into(), id.into()));
        Ok(())
    }

    // ── what the window asks for ───────────────────────────────────────────

    pub fn snapshot(&self) -> Snapshot {
        let state = self.inner.state.lock().unwrap();
        self.picture(&state)
    }

    pub fn model_catalog(&self, id: &str, cwd: &str, profile: &str, refresh: bool) -> Result<crate::catalog::Catalog, String> {
        let kind = self.kind(id)?.clone();
        crate::launch::Options { profile: profile.into(), ..Default::default() }.validate(kind.launch)?;
        let found = self.locate(&kind)?;
        Ok(self.inner.catalogs.get(&kind, &found, cwd, profile, refresh))
    }

    /// Seat a new agent and start its program. Returns the desk's id.
    pub fn new_agent(&self, spec: NewAgent, cols: u16, rows: u16) -> Result<String, String> {
        self.new_agent_id(spec, cols, rows, new_id(), None)
    }

    pub(crate) fn new_agent_id(&self, spec: NewAgent, cols: u16, rows: u16, id: String, generated_title: Option<&str>) -> Result<String, String> {
        if self.inner.stopping.load(Ordering::Relaxed) { return Err("The office is shutting down.".into()); }
        let kind = self.kind(&spec.harness)?.clone();
        let cwd = spec.cwd.trim().to_string();
        if cwd.is_empty() || !Path::new(&cwd).is_dir() {
            return Err("That folder does not exist. Choose the folder the agent should work in.".into());
        }
        let found = self.locate(&kind)?;
        let task = spec.prompt.trim();
        let named = !spec.title.trim().is_empty();
        let auto = if task.is_empty() { format!("{} in {}", kind.tag, status::base_name(&cwd)) } else { title_from(task) };
        // Two agents in one folder can be told apart: "Claude in web", "Claude in web 2".
        let auto = self.inner.state.lock().unwrap().office.unique_title(&auto);
        let title = if named { harness::shorten(&spec.title, 60) } else { auto.clone() };
        let session = (kind.session == SessionFrom::Given).then(new_uuid);
        let args = crate::launch::start_args(&kind, session.as_deref(), if named { &title } else { "" }, task, spec.worktree, &spec.launch)?;
        if spec.worktree && kind.worktree.is_empty() { return Err("This program does not support worktrees.".into()); }
        if generated_title.is_some() && found.through_shell {
            return Err("This wrapper cannot receive MCP configuration safely. Use the native Claude Code or Codex executable for delegation.".into());
        }
        let (args, env) = if found.through_shell { (args, vec![]) } else { self.mcp_launch(&kind, &id, args, &spec.launch)? };
        let (program, args) = match harness::command_line(&found, &args) {
            Ok(command) => command,
            Err(error) => { self.mcp_revoke(&id); return Err(error); }
        };
        let desk = SavedDesk { id: id.clone(), harness: kind.id.clone(), title, title_locked: named, auto_title: auto, cwd: cwd.clone(), session_verified: session.is_some(), session, created_ms: now_ms(), launch: spec.launch };
        // Register the desk before spawning: a fast MCP startup or process exit
        // must already be able to identify it. Never hold the state lock to spawn.
        {
            let mut state = self.inner.state.lock().unwrap();
            state.office.add(desk, now_ms());
            // Labels suggested by a requesting agent are set before the child
            // starts, remain editable, and never acquire the user's name lock.
            if let Some(title) = generated_title.filter(|s| !s.trim().is_empty()) { state.office.agent_title(&id, title); }
            state.office.started(&id, !task.is_empty(), false, now_ms());
            self.place(&mut state, &id);
            self.publish(&mut state);
        }
        if let Err(error) = self.inner.terms.start(&id, Launch { program, args, cwd, cols, rows, env }) {
            self.mcp_revoke(&id);
            let mut state = self.inner.state.lock().unwrap();
            state.office.remove(&id);
            self.publish(&mut state);
            return Err(error);
        }
        Ok(id)
    }

    /// Make sure a desk's program is running: someone asleep is woken, carrying
    /// on where they left off when their program can.
    pub fn wake(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        if self.inner.terms.running(id) {
            return Ok(());
        }
        let (kind, cwd, session, launch) = {
            let state = self.inner.state.lock().unwrap();
            let desk = state.office.get(id).ok_or("That desk is gone.")?;
            let kind = self.kind(&desk.saved.harness)?;
            (kind.clone(), desk.saved.cwd.clone(), desk.saved.resume_session(kind).map(str::to_string), desk.saved.launch.clone())
        };
        if cwd.is_empty() || !Path::new(&cwd).is_dir() {
            return Err(format!("The saved folder no longer exists: {cwd}. Restore it, or start a new desk in the project's new folder."));
        }
        let found = self.locate(&kind)?;
        // Carry on when it can; otherwise a fresh start in the same folder.
        let (args, fresh) = match crate::launch::resume_args(&kind, session.as_deref(), &launch)? {
            Some(args) => (args, None),
            None => {
                let fresh = (kind.session == SessionFrom::Given).then(new_uuid);
                (crate::launch::start_args(&kind, fresh.as_deref(), "", "", false, &launch)?, fresh)
            }
        };
        let (args, env) = if found.through_shell { (args, vec![]) } else { self.mcp_launch(&kind, id, args, &launch)? };
        let (program, args) = match harness::command_line(&found, &args) {
            Ok(command) => command,
            Err(error) => { self.mcp_revoke(id); return Err(error); }
        };

        let outcome = self.inner.terms.start(id, Launch { program, args, cwd, cols, rows, env });
        let mut state = self.inner.state.lock().unwrap();
        let now = now_ms();
        let result = match outcome {
            Ok(_) => {
                if let Some(fresh) = fresh {
                    state.office.set_session(id, &fresh);
                }
                state.office.started(id, false, false, now);
                Ok(())
            }
            Err(why) => {
                self.mcp_revoke(id);
                state.office.failed_to_start(id, &why, now);
                Err(why)
            }
        };
        self.publish(&mut state);
        result
    }

    /// Install a program, or update it, in a terminal of its own. Returns the
    /// terminal's id; one already running for the same thing is returned as it is.
    pub fn run_job(&self, harness_id: &str, kind: JobKind, cols: u16, rows: u16) -> Result<String, String> {
        let program = self.kind(harness_id)?.clone();
        let id = format!("job-{}-{}", kind.word(), program.id);
        {
            let state = self.inner.state.lock().unwrap();
            if state.jobs.iter().any(|j| j.id == id && j.running) {
                return Ok(id);
            }
        }
        let runner = match kind {
            JobKind::Install => harness::install_runner(&program),
            JobKind::Update => harness::update_runner(&program),
            JobKind::SignIn => harness::login_runner(&program),
        };
        let Some(runner) = runner else {
            return Err(match kind {
                JobKind::Install => format!("The office does not know how to install {}. Install it the way its makers say, then it will be offered here.", program.name),
                JobKind::Update => format!("The office does not know how to update {}.", program.name),
                JobKind::SignIn => format!("The office does not know how to sign in to {}. Start one of its desks and sign in from its terminal.", program.name),
            });
        };
        let (found, words) = match runner {
            Runner::Npm(words) => {
                let npm = harness::find("npm").ok_or_else(|| format!("{} is installed with npm, which was not found. Install Node.js from nodejs.org, then try again.", program.name))?;
                (npm, words)
            }
            Runner::Itself(words) => (self.locate(&program)?, harness::own_words(&program, &words)),
            Runner::Command(words) => {
                let first = words.first().cloned().unwrap_or_default();
                let found = harness::find(&first).ok_or_else(|| format!("`{first}`, which installs {}, was not found.", program.name))?;
                (found, words[1..].to_vec())
            }
        };
        if kind == JobKind::Install && harness::find(&program.program).is_some() {
            return Err(format!("{} is already installed.", program.name));
        }
        let (exe, args) = harness::command_line(&found, &words)?;
        let cwd = self.inner.home.to_string_lossy().into_owned();
        // A job that ran before under this id is replaced, terminal and all.
        self.inner.terms.start(&id, Launch { program: exe, args, cwd, cols, rows, env: vec![] })?;
        let mut state = self.inner.state.lock().unwrap();
        state.jobs.retain(|j| j.id != id);
        state.jobs.push(Job { id: id.clone(), harness: program.id.clone(), kind, running: true, ok: None });
        self.publish(&mut state);
        Ok(id)
    }

    /// Put a job's terminal away for good. A job still running is ended.
    pub fn forget_job(&self, id: &str) {
        self.inner.terms.remove(id);
        let mut state = self.inner.state.lock().unwrap();
        state.jobs.retain(|j| j.id != id);
        self.publish(&mut state);
    }

    pub fn attach(&self, id: &str, sink: Sink) -> Result<u64, String> {
        self.inner.terms.attach(id, sink)
    }

    pub fn detach(&self, id: &str, token: u64) {
        self.inner.terms.detach(id, token);
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        self.inner.terms.write(id, data)?;
        // The terminal answering for itself (focus, the cursor's place) is not someone typing.
        if !status::is_report(data) {
            self.inner.state.lock().unwrap().office.typed(id, now_ms());
        }
        Ok(())
    }

    pub fn voice_target(&self, id: &str) -> Option<crate::voice::Target> {
        let state = self.inner.state.lock().unwrap();
        if !state.visible.contains(id) || !state.office.get(id)?.running { return None; }
        Some(crate::voice::Target { agent: id.into(), run: self.inner.terms.generation(id)? })
    }

    pub fn voice_live(&self, target: &crate::voice::Target) -> bool {
        self.voice_target(&target.agent).as_ref() == Some(target)
    }

    /// Words a dictation into this desk is likely to contain, for a cloud provider to expect:
    /// the desk's name, its folder, project and branch, and the names of the programs here.
    pub fn voice_context(&self, id: &str, desk: bool) -> Vec<String> {
        let state = self.inner.state.lock().unwrap();
        let mut words = Vec::new();
        if desk {
            if let Some(d) = state.office.get(id) {
                let folder = std::path::Path::new(&d.saved.cwd).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                words.extend([d.saved.title.clone(), folder, d.repo.clone(), d.branch.clone()]);
            }
        }
        for kind in self.inner.table.iter() {
            words.extend([kind.name.clone(), kind.tag.clone()]);
        }
        words
    }

    /// Types dictated words as plain text. `space` puts a space before them, between words
    /// this recording already typed and these.
    pub fn voice_write(&self, target: &crate::voice::Target, data: &[u8], space: bool) -> Result<(), String> {
        let text = crate::voice::sanitize(std::str::from_utf8(data).map_err(|_| "The voice text was invalid and discarded.")?);
        if text.is_empty() || text.len() > 32 * 1024 { return Err("The voice text was empty or too long and discarded.".into()); }
        let text = if space { format!(" {text}") } else { text };
        let mut state = self.inner.state.lock().unwrap();
        if !state.visible.contains(&target.agent) || !state.office.get(&target.agent).is_some_and(|d| d.running) {
            return Err("The recording's pane closed or its program stopped. Its text was discarded.".into());
        }
        self.inner.terms.write_generation(&target.agent, target.run, text.as_bytes())?;
        state.voice_used.insert(target.agent.clone());
        state.office.typed(&target.agent, now_ms());
        Ok(())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) {
        self.inner.terms.resize(id, cols, rows);
    }

    /// Capture a final CLI session switch before stop/exit can leave an older ID saved.
    fn final_session(&self, state: &mut State, id: &str) {
        let Some(desk) = state.office.get(id) else { return };
        if !self.kind(&desk.saved.harness).is_ok_and(|h| h.session == SessionFrom::CodexRollouts) {
            return;
        }
        let Some((_, title, _)) = self.inner.terms.signal(id, now_ms()) else { return };
        if status::session_prefix(&title).is_none() || (desk.saved.session_verified && desk.saved.session.as_deref().is_some_and(|s| status::session_title_matches(&title, s))) {
            return;
        }
        state.office.unverify_session(id);
        if let Some(session) = status::codex_session(&self.inner.home, &title) {
            state.office.set_session(id, &session);
        }
    }

    /// End a desk's program. The desk stays, asleep.
    pub fn stop(&self, id: &str) {
        self.mcp_revoke(id);
        {
            let mut state = self.inner.state.lock().unwrap();
            self.final_session(&mut state, id);
            state.office.will_stop(id);
        }
        self.inner.terms.stop(id);
    }

    /// End a desk's program and start it again, carrying on where it left off when it can.
    pub fn restart(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        self.stop(id);
        // Its end has to be seen first, or the new program would be taken for the old one ending.
        for _ in 0..100 {
            let gone = !self.inner.terms.running(id) && !self.inner.state.lock().unwrap().office.get(id).is_some_and(|d| d.running);
            if gone {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        self.wake(id, cols, rows)
    }

    /// The folder a desk works in, if it is one that exists.
    pub fn folder(&self, id: &str) -> Option<PathBuf> {
        let cwd = self.inner.state.lock().unwrap().office.get(id)?.saved.cwd.clone();
        let path = PathBuf::from(cwd);
        path.is_dir().then_some(path)
    }

    /// What to say before quitting, when quitting would end someone's work. None when nobody is busy.
    pub fn quit_words(&self) -> Option<String> {
        let state = self.inner.state.lock().unwrap();
        let n = state.office.busy();
        let jobs: Vec<_> = state.jobs.iter().filter(|j| j.running).map(|j| format!("{} {}", self.kind(&j.harness).map_or(j.harness.as_str(), |h| h.name.as_str()), j.kind.name())).collect();
        let job_warning = if jobs.is_empty() { String::new() } else { format!("Still running: {}. Quitting interrupts {}; you may need to run {} again.", jobs.join(", "), if jobs.len() == 1 { "it" } else { "them" }, if jobs.len() == 1 { "it" } else { "them" }) };
        if n == 0 {
            return (!jobs.is_empty()).then_some(job_warning);
        }
        let busy: Vec<_> = state.office.desks().iter().filter(|d| d.running && matches!(d.phase, Phase::Working | Phase::Quiet | Phase::NeedsYou | Phase::Starting)).collect();
        let afresh: Vec<String> = busy.iter().filter(|d| !self.kind(&d.saved.harness).is_ok_and(|h| harness::can_resume(h, d.saved.resume_session(h)))).map(|d| d.saved.title.clone()).collect();
        let head = if n == 1 { "1 agent is still busy. Quitting ends its program.".to_string() } else { format!("{n} agents are still busy. Quitting ends their programs.") };
        let tail = match (afresh.len(), n) {
            (0, _) => " Conversations stay in the programs' history. Each desk shows whether it resumes its own conversation or the program's latest one.".to_string(),
            (1, 1) => " Its program cannot carry on a conversation, so it starts afresh next time.".to_string(),
            (a, b) if a == b => " Their programs cannot carry on a conversation, so they start afresh next time.".to_string(),
            _ => format!(" {} will start afresh next time; the other desks show which conversation their programs resume.", afresh.join(", ")),
        };
        Some(format!("{head}{tail}{}", if job_warning.is_empty() { String::new() } else { format!(" {job_warning}") }))
    }

    /// Take the desk away, ending its program. The conversation is the program's to keep.
    pub fn dismiss(&self, id: &str) {
        self.mcp_revoke(id);
        // What it was asked to do by another session will not be reported now.
        if let Some(hub) = self.inner.mcp.lock().unwrap().as_ref() {
            hub.abandoned(id);
        }
        self.inner.terms.remove(id);
        let mut state = self.inner.state.lock().unwrap();
        state.office.remove(id);
        state.watched.remove(id);
        self.publish(&mut state);
    }

    /// Name a desk. An empty name gives back the name it was given when it was seated.
    pub fn rename(&self, id: &str, title: &str) {
        let mut state = self.inner.state.lock().unwrap();
        // A desk kept from before names were kept has the plain one: its program and folder.
        let fallback = state.office.get(id).map(|d| format!("{} in {}", self.kind(&d.saved.harness).map_or("Agent", |h| h.tag.as_str()), status::base_name(&d.saved.cwd))).unwrap_or_default();
        if state.office.rename(id, title, &fallback) {
            self.publish(&mut state);
        }
    }

    /// Which terminals are being read, and which panes remain visible for voice.
    pub fn watch(&self, ids: Vec<String>, visible: Vec<String>) {
        let mut state = self.inner.state.lock().unwrap();
        state.watched = ids.into_iter().collect();
        state.visible = visible.into_iter().collect();
        if state.focused {
            self.see(&mut state);
        }
    }

    /// The window came to the front, went behind, or closed.
    pub fn attention(&self, visible: bool, focused: bool) {
        let mut state = self.inner.state.lock().unwrap();
        let came = visible && focused && !state.focused;
        state.focused = visible && focused;
        if !visible {
            state.watched.clear();
            state.visible.clear();
        }
        if state.focused {
            self.see(&mut state);
        }
        // Back from signing in somewhere else, a browser say: a program believed signed out is asked again.
        let out: Vec<String> = if came { self.signed_out(&state) } else { Vec::new() };
        drop(state);
        for id in out {
            self.wonder(&id);
        }
    }

    /// End every program. Called once, on the way out.
    pub fn shutdown(&self) {
        self.inner.stopping.store(true, Ordering::Relaxed);
        if let Some(hub) = self.inner.mcp.lock().unwrap().as_ref() { hub.shutdown(); }
        self.inner.terms.stop_all();
        let mut state = self.inner.state.lock().unwrap();
        let ids: Vec<String> = state.office.desks().iter().map(|d| d.saved.id.clone()).collect();
        for id in ids {
            self.final_session(&mut state, &id);
        }
        state.revision += 1;
        let revision = state.revision;
        let saved = self.to_save(&state);
        // Only desks' screens: an install's terminal is not kept.
        let mut screens: Vec<(String, Vec<u8>)> = self.inner.terms.screens(SCREEN_HISTORY).into_iter().filter(|(id, _)| state.office.get(id).is_some() && !state.voice_used.contains(id)).collect();
        // And those of the desks held aside, as they were: a screen not handed over is deleted.
        screens.extend(self.inner.aside.iter().filter_map(|a| Some((a.desk.id.clone(), a.screen.clone()?))));
        drop(state);
        // Saving can report a storage error to the desktop; never hold the engine lock for that.
        self.inner.shell.save(revision, &saved);
        self.inner.shell.save_screens(&screens);
    }

    // ── what happens by itself ─────────────────────────────────────────────

    fn exited(&self, id: &str, ok: bool) {
        self.mcp_revoke(id);
        let mut state = self.inner.state.lock().unwrap();
        if let Some(job) = state.jobs.iter_mut().find(|j| j.id == id) {
            job.running = false;
            job.ok = Some(ok);
            let harness = job.harness.clone();
            self.publish(&mut state);
            drop(state);
            // The program may be somewhere new, or a new version: look again, off this thread.
            let again = self.clone();
            std::thread::spawn(move || again.find_program(&harness));
            return;
        }
        self.final_session(&mut state, id);
        let kind = state.office.get(id).map(|d| d.saved.harness.clone());
        let change = state.office.exited(id, ok, now_ms());
        self.tell(&state, change);
        self.publish(&mut state);
        drop(state);
        // Signing out ends a program, in any terminal. Its desk says so once it is known.
        if let Some(kind) = kind {
            self.wonder(&kind);
        }
    }

    /// One look at every running program.
    fn look(&self) {
        let pids = self.inner.terms.pids();
        let now = now_ms();
        let mut state = self.inner.state.lock().unwrap();
        state.looks += 1;
        let slow = state.looks.is_multiple_of(SLOW_EVERY);
        for (id, pid) in pids {
            let Some(desk) = state.office.get(&id) else {
                continue;
            };
            let Ok(kind) = self.kind(&desk.saved.harness) else {
                continue;
            };
            let session = desk.saved.resume_session(kind).map(str::to_string);
            let mut cwd = desk.saved.cwd.clone();
            let Some((mut signal, title, printed)) = self.inner.terms.signal(&id, now) else {
                continue;
            };
            state.office.observed_trust(&id, self.inner.terms.trust_prompt(&id, kind.trust));

            if kind.status == StatusFrom::ClaudeSessions {
                if let Some(session) = status::read_claude_session(&self.inner.home, pid) {
                    // What Claude says of itself is better than what its terminal suggests,
                    // except before it has drawn anything.
                    if signal != Signal::Silent {
                        signal = status::claude_signal(&session).unwrap_or(signal);
                    }
                    // The name it makes up for a conversation nobody named is not a name for the desk.
                    state.office.session_named(&id, &session.name, session.name_source != "derived");
                    state.office.set_session(&id, &session.session_id);
                    if slow && !session.cwd.is_empty() {
                        cwd = session.cwd;
                    }
                }
            }
            if kind.session == SessionFrom::CodexRollouts && !session.as_deref().is_some_and(|s| status::session_title_matches(&title, s)) {
                if status::session_prefix(&title).is_some() {
                    state.office.unverify_session(&id);
                }
                if slow {
                    if let Some(session) = status::codex_session(&self.inner.home, &title) {
                        state.office.set_session(&id, &session);
                    }
                }
            }
            if slow {
                let (project, repo, branch) = status::project(Path::new(&cwd));
                state.office.set_place(&id, project, repo, branch);
            }
            // A desk still under the name it was seated with takes the topic its program
            // has put in its terminal's title, once it has been given something to do.
            if kind.topic == harness::TopicFrom::TerminalTitle {
                if let Some(topic) = status::topic(&title, &[&kind.name, &kind.tag, &kind.program]) {
                    state.office.topic(&id, &topic);
                }
            }
            let watched = state.focused && state.watched.contains(&id);
            // Before the look: what it printed while it was still drawing its opening screen is not news.
            state.office.heard(&id, printed, watched, now);
            let change = state.office.observe(&id, &signal, watched, now);
            self.tell(&state, change);
        }
        self.publish(&mut state);
        if state.looks.is_multiple_of(ACCOUNT_EVERY) {
            // Signing in ends nothing, so there is no moment to ask at: asked now and then,
            // and only while one of the program's desks, or its sign-in, is running.
            let busy: Vec<String> = self
                .signed_out(&state)
                .into_iter()
                .filter(|kind| state.office.desks().iter().any(|d| d.running && &d.saved.harness == kind) || state.jobs.iter().any(|j| j.running && j.kind == JobKind::SignIn && &j.harness == kind))
                .collect();
            drop(state);
            for kind in busy {
                self.wonder(&kind);
            }
        }
    }

    /// Flags come down at the desks whose terminals are on screen.
    fn see(&self, state: &mut State) {
        let now = now_ms();
        let watched: Vec<String> = state.watched.iter().cloned().collect();
        let mut any = false;
        for id in watched {
            any |= state.office.mark_seen(&id, now);
        }
        if any {
            self.publish(state);
        }
    }

    /// Tell the user about a change they would want to hear of while looking elsewhere.
    fn tell(&self, state: &State, change: Option<Transition>) {
        let Some(change) = change else { return };
        if state.focused && state.watched.contains(&change.id) {
            return;
        }
        match change.to {
            Phase::NeedsYou => {
                let body = if change.note.is_empty() { "Is waiting for you in their terminal." } else { change.note.as_str() };
                self.say(Out::Notify(format!("{} needs you", change.title), body.to_string(), change.id.clone()));
            }
            Phase::Done => {
                let body = if change.note.is_empty() { "Open their terminal to see what they did." } else { change.note.as_str() };
                self.say(Out::Notify(format!("{} is done", change.title), body.to_string(), change.id.clone()));
            }
            Phase::Failed => {
                let body = if change.note.is_empty() { "Something went wrong." } else { change.note.as_str() };
                self.say(Out::Notify(format!("{} hit a problem", change.title), body.to_string(), change.id.clone()));
            }
            _ => {}
        }
    }

    /// Work out which project and branch a desk's folder is.
    fn place(&self, state: &mut State, id: &str) {
        let Some(cwd) = state.office.get(id).map(|d| d.saved.cwd.clone()) else {
            return;
        };
        let (project, repo, branch) = status::project(Path::new(&cwd));
        state.office.set_place(id, project, repo, branch);
    }

    fn refresh_places(&self) {
        let mut state = self.inner.state.lock().unwrap();
        let ids: Vec<String> = state.office.desks().iter().map(|d| d.saved.id.clone()).collect();
        for id in ids {
            self.place(&mut state, &id);
        }
    }

    /// Send the window the office as it stands, if it differs from what it has.
    fn publish(&self, state: &mut State) {
        if self.inner.stopping.load(Ordering::Relaxed) {
            return;
        }
        let picture = self.picture(state);
        let now = (picture.agents.clone(), picture.harnesses.clone(), picture.jobs.clone());
        if state.told.as_ref() != Some(&now) {
            state.told = Some(now);
            self.say(Out::Snapshot(picture));
        }
        let saved = self.to_save(state);
        if saved != state.saved {
            state.revision += 1;
            self.say(Out::Save(state.revision, saved.clone()));
            state.saved = saved;
        }
    }

    /// The desks to keep on disk: everyone in the office, and those held aside, each
    /// put back where it was in the list, which is the order the floor seats them in.
    fn to_save(&self, state: &State) -> Vec<SavedDesk> {
        let mut desks = state.office.saved();
        for aside in &self.inner.aside {
            desks.insert(aside.place.min(desks.len()), aside.desk.clone());
        }
        desks
    }

    /// Queue something for the desktop. Never waits.
    fn say(&self, message: Out) {
        let _ = self.inner.out.lock().unwrap().send(message);
    }

    fn picture(&self, state: &State) -> Snapshot {
        let harnesses = self
            .inner
            .table
            .iter()
            .map(|h| {
                let here = state.installed.get(&h.id);
                let installed = here.is_some_and(|i| i.found.is_some());
                let version = here.map(|i| i.version.clone()).unwrap_or_default();
                let latest = state.latest.get(&h.id).cloned().unwrap_or_default();
                HarnessView {
                    id: h.id.clone(),
                    name: h.name.clone(),
                    tag: h.tag.clone(),
                    installed,
                    launch: h.launch,
                    outdated: installed && harness::newer(&version, &latest),
                    version,
                    latest,
                    takes_task: h.task != harness::TaskArg::None,
                    worktree: !h.worktree.is_empty(),
                    install_line: harness::install_runner(h).map(|r| r.line(&h.program)).unwrap_or_default(),
                    update_line: harness::update_runner(h).map(|r| r.line(&h.program)).unwrap_or_default(),
                    signed_in: here.filter(|_| installed).and_then(|i| i.account.signed_in),
                    account: here.filter(|_| installed).map(|i| i.account.label.clone()).unwrap_or_default(),
                    login_line: harness::login_runner(h).map(|r| r.line(&h.program)).unwrap_or_default(),
                }
            })
            .collect();
        let jobs = state
            .jobs
            .iter()
            .map(|j| JobView { id: j.id.clone(), harness: j.harness.clone(), harness_name: self.kind(&j.harness).map_or_else(|_| j.harness.clone(), |h| h.name.clone()), kind: j.kind.word().to_string(), running: j.running, ok: j.ok })
            .collect();
        Snapshot { agents: state.office.views(&self.inner.table), harnesses, jobs, now_ms: now_ms() }
    }

    // ── the programs on this computer ──────────────────────────────────────

    fn kind(&self, id: &str) -> Result<&Harness, String> {
        self.inner.table.iter().find(|h| h.id == id).ok_or_else(|| "That kind of agent is not in the table any more.".to_string())
    }

    /// Where a program is. Looked for again each time, so one installed while the
    /// office is open is found without restarting it.
    fn locate(&self, kind: &Harness) -> Result<Found, String> {
        let found = harness::find(&kind.program);
        let mut state = self.inner.state.lock().unwrap();
        let entry = state.installed.entry(kind.id.clone()).or_default();
        let changed = entry.found != found;
        entry.found = found.clone();
        if changed {
            self.publish(&mut state);
        }
        found.ok_or_else(|| format!("{} was not found on this computer. Install it, or check that `{}` runs in a terminal.", kind.name, kind.program))
    }

    /// Look for one program again, and ask its version.
    fn find_program(&self, harness_id: &str) {
        let Ok(kind) = self.kind(harness_id) else {
            return;
        };
        let found = harness::find(&kind.program);
        let version = found.as_ref().map(harness::version).unwrap_or_default();
        let account = found.as_ref().and_then(|found| Self::account_of(kind, found));
        let mut state = self.inner.state.lock().unwrap();
        let entry = state.installed.entry(harness_id.to_string()).or_default();
        entry.found = found;
        entry.version = version;
        if let Some(account) = account {
            entry.account = account;
        }
        self.publish(&mut state);
    }

    /// Who is signed in to a program, asked of the program itself. None when it has no
    /// way of saying, did not answer in time, or said something that was not understood:
    /// what was known before then stands. This starts the program and waits for it, so it
    /// is never called under the engine's lock.
    fn account_of(kind: &Harness, found: &Found) -> Option<harness::Account> {
        if kind.account.is_empty() {
            return None;
        }
        let said = harness::answer(found, &harness::own_words(kind, &kind.account), 15)?;
        Some(harness::account_from(&said)).filter(|account| account.signed_in.is_some())
    }

    /// Ask one program who is signed in to it, and tell the window if that has changed.
    fn ask_account(&self, harness_id: &str) {
        let Ok(kind) = self.kind(harness_id) else {
            return;
        };
        let found = self.inner.state.lock().unwrap().installed.get(harness_id).and_then(|i| i.found.clone());
        let Some(account) = found.and_then(|found| Self::account_of(kind, &found)) else {
            return;
        };
        let mut state = self.inner.state.lock().unwrap();
        state.installed.entry(harness_id.to_string()).or_default().account = account;
        self.publish(&mut state);
    }

    /// Have a program asked who is signed in to it, off this thread. One that cannot
    /// say is not asked, and one already waiting to be asked is not asked twice.
    fn wonder(&self, harness_id: &str) {
        if self.inner.stopping.load(Ordering::Relaxed) || !self.kind(harness_id).is_ok_and(|kind| !kind.account.is_empty()) {
            return;
        }
        if self.inner.asking.lock().unwrap().insert(harness_id.to_string()) {
            let _ = self.inner.accounts.lock().unwrap().send(harness_id.to_string());
        }
    }

    /// The programs on this computer that last said nobody is signed in to them.
    fn signed_out(&self, state: &State) -> Vec<String> {
        state.installed.iter().filter(|(_, here)| here.found.is_some() && here.account.signed_in == Some(false)).map(|(id, _)| id.clone()).collect()
    }

    /// Ask every program that can say who is signed in, again: someone is looking at them.
    pub fn refresh_accounts(&self) {
        let here: Vec<String> = {
            let state = self.inner.state.lock().unwrap();
            self.inner.table.iter().filter(|h| !h.account.is_empty() && state.installed.get(&h.id).is_some_and(|i| i.found.is_some())).map(|h| h.id.clone()).collect()
        };
        for id in here {
            self.wonder(&id);
        }
    }

    /// Ask npm for the newest version of every installed program it publishes.
    fn check_updates(&self) {
        let Some(npm) = harness::find("npm") else {
            return;
        };
        let asks: Vec<(String, String)> = {
            let state = self.inner.state.lock().unwrap();
            self.inner
                .table
                .iter()
                .filter(|h| !h.package.is_empty() && state.installed.get(&h.id).is_some_and(|i| i.found.is_some()))
                .map(|h| (h.id.clone(), h.package.clone()))
                .collect()
        };
        for (id, package) in asks {
            if self.inner.stopping.load(Ordering::Relaxed) {
                return;
            }
            let latest = harness::latest(&npm, &package);
            if latest.is_empty() {
                continue;
            }
            let mut state = self.inner.state.lock().unwrap();
            state.latest.insert(id, latest);
            self.publish(&mut state);
        }
    }

    /// Look for every program of the table, then ask the ones found their version.
    fn find_programs(&self) {
        let found: Vec<(String, Option<Found>)> = self.inner.table.iter().map(|h| (h.id.clone(), harness::find(&h.program))).collect();
        {
            let mut state = self.inner.state.lock().unwrap();
            for (id, found) in &found {
                state.installed.entry(id.clone()).or_default().found = found.clone();
            }
            self.publish(&mut state);
        }
        for (id, found) in found {
            let Some(found) = found else { continue };
            if self.inner.stopping.load(Ordering::Relaxed) {
                return;
            }
            let version = harness::version(&found);
            // Asked in the same pass and told in the same breath: once the last program has a
            // version, the office has started nothing more of its own.
            let account = self.kind(&id).ok().and_then(|kind| Self::account_of(kind, &found));
            let mut state = self.inner.state.lock().unwrap();
            let entry = state.installed.entry(id).or_default();
            entry.version = version;
            if let Some(account) = account {
                entry.account = account;
            }
            self.publish(&mut state);
        }
    }
}

/// Sixteen hex digits nobody else has.
fn new_id() -> String {
    let mut bytes = [0u8; 8];
    random(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A version 4 UUID, which is what Claude Code wants a conversation's id to be.
fn new_uuid() -> String {
    let mut b = [0u8; 16];
    random(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

fn random(bytes: &mut [u8]) {
    if getrandom::fill(bytes).is_err() {
        // No source of randomness: the clock and the address of a local are different enough for an id.
        let seed = now_ms() as u128 ^ (bytes.as_ptr() as usize as u128) << 32;
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (seed >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(151);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{SessionFrom, StatusFrom, TaskArg, TrustFrom};

    #[derive(Default)]
    struct Quiet {
        snapshots: Mutex<Vec<Snapshot>>,
        notices: Mutex<Vec<(String, String)>>,
        saved: Mutex<Vec<SavedDesk>>,
        screens: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl Shell for Quiet {
        fn snapshot(&self, snapshot: &Snapshot) {
            self.snapshots.lock().unwrap().push(snapshot.clone());
        }
        fn notify(&self, title: &str, body: &str, _desk: &str) {
            self.notices.lock().unwrap().push((title.into(), body.into()));
        }
        fn save(&self, _revision: u64, desks: &[SavedDesk]) {
            *self.saved.lock().unwrap() = desks.to_vec();
        }
        fn save_screens(&self, screens: &[(String, Vec<u8>)]) {
            *self.screens.lock().unwrap() = screens.to_vec();
        }
    }

    /// A stand-in agent: the system's own shell, which every computer has.
    fn shell_kind() -> Harness {
        let (program, args) = if cfg!(windows) { ("cmd.exe", vec!["/d".to_string(), "/q".to_string()]) } else { ("/bin/sh", vec![]) };
        Harness {
            id: "shell".into(),
            launch: crate::launch::Provider::None,
            name: "A Shell".into(),
            tag: "Shell".into(),
            program: program.into(),
            args,
            task: TaskArg::None,
            resume: vec![],
            session: SessionFrom::Unknown,
            session_arg: String::new(),
            name_arg: String::new(),
            worktree: vec![],
            status: StatusFrom::Activity,
            trust: TrustFrom::None,
            topic: harness::TopicFrom::None,
            package: String::new(),
            update: vec![],
            install: vec![],
            login: vec![],
            account: vec![],
        }
    }

    fn until(mut ready: impl FnMut() -> bool) -> bool {
        for _ in 0..160 {
            if ready() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn an_agent_is_seated_runs_is_stopped_and_woken_again() {
        let shell = Arc::new(Quiet::default());
        let engine = start(shell.clone(), vec![shell_kind()], vec![], vec![]);
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        let spec = NewAgent { harness: "shell".into(), cwd: cwd.clone(), prompt: String::new(), title: "Probe".into(), worktree: false, launch: Default::default() };
        let id = engine.new_agent(spec, 80, 24).unwrap();

        let agent = |engine: &Handle| engine.snapshot().agents.into_iter().find(|a| a.id == id).unwrap();
        assert_eq!((agent(&engine).title.as_str(), agent(&engine).harness_tag.as_str(), agent(&engine).running), ("Probe", "Shell", true));
        // A shell prints its prompt and then waits: it settles as idle, with no flag for having done nothing.
        assert!(until(|| agent(&engine).phase == Phase::Idle), "{:?}", agent(&engine).phase);
        assert!(until(|| shell.saved.lock().unwrap().len() == 1));
        assert!(until(|| shell.snapshots.lock().unwrap().iter().any(|s| s.agents.len() == 1)));
        assert!(shell.notices.lock().unwrap().is_empty());

        // Typed into, it answers; watched from a terminal, that is seen.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        engine
            .attach(
                &id,
                Box::new(move |bytes: &[u8]| {
                    into.lock().unwrap().extend_from_slice(bytes);
                    true
                }),
            )
            .unwrap();
        engine.write(&id, b"echo from-the-desk\r\n").unwrap();
        assert!(until(|| String::from_utf8_lossy(&seen.lock().unwrap()).contains("from-the-desk")));

        engine.stop(&id);
        assert!(until(|| agent(&engine).phase == Phase::Asleep), "{:?}", agent(&engine).phase);
        assert!(!agent(&engine).running);
        engine.wake(&id, 80, 24).unwrap();
        assert!(agent(&engine).running);

        engine.dismiss(&id);
        assert!(engine.snapshot().agents.is_empty());
        assert!(until(|| shell.saved.lock().unwrap().is_empty()));
        engine.shutdown();
    }

    #[test]
    fn voice_pins_a_live_run_rejects_stale_or_hidden_panes_and_does_not_save_echo() {
        let shell = Arc::new(Quiet::default());
        let engine = start(shell.clone(), vec![shell_kind()], vec![], vec![]);
        let spec = NewAgent { harness: "shell".into(), cwd: std::env::temp_dir().to_string_lossy().into_owned(), prompt: String::new(), title: "Voice fixture".into(), worktree: false, launch: Default::default() };
        let id = engine.new_agent(spec, 80, 24).unwrap();
        assert!(engine.voice_target(&id).is_none());
        engine.watch(vec![id.clone()], vec![id.clone()]);
        let target = engine.voice_target(&id).unwrap();
        assert!(engine.voice_live(&target));
        // Selecting another pane must not move or discard this recording's target.
        engine.watch(vec![], vec![id.clone()]);
        assert!(engine.voice_live(&target));
        assert!(engine.voice_write(&target, b"focus-change", false).is_ok());
        let invalid = crate::voice::Target { run: target.run + 100, ..target.clone() };
        assert!(engine.voice_write(&invalid, b"must-not-reach-the-terminal", false).is_err());
        engine.watch(vec![], vec![]);
        assert!(!engine.voice_live(&target));
        assert!(engine.voice_write(&target, b"also-discarded", false).is_err());
        engine.watch(vec![id.clone()], vec![id.clone()]);
        assert!(engine.voice_write(&target, b"voice\r\ntext\x1b[31m\x03", true).is_ok());
        engine.restart(&id, 80, 24).unwrap();
        assert!(!engine.voice_live(&target));
        assert!(engine.voice_write(&target, b"stale-result", false).is_err());
        assert_ne!(engine.voice_target(&id).unwrap().run, target.run);
        engine.shutdown();
        assert!(shell.screens.lock().unwrap().is_empty());
    }

    #[test]
    fn an_agent_is_not_seated_without_a_folder_or_a_program() {
        let shell = Arc::new(Quiet::default());
        let mut missing = shell_kind();
        missing.id = "ghost".into();
        missing.name = "Ghost".into();
        missing.program = "no-such-program-moshpit".into();
        let engine = start(shell, vec![shell_kind(), missing], vec![], vec![]);
        let here = std::env::temp_dir().to_string_lossy().into_owned();
        let spec = |harness: &str, cwd: &str| NewAgent { harness: harness.into(), cwd: cwd.into(), prompt: String::new(), title: String::new(), worktree: false, launch: Default::default() };

        assert!(engine.new_agent(spec("shell", "Z:/no/such/folder/anywhere"), 80, 24).unwrap_err().contains("folder"));
        assert!(engine.new_agent(spec("ghost", &here), 80, 24).unwrap_err().contains("Ghost was not found"));
        assert!(engine.new_agent(spec("nobody", &here), 80, 24).is_err());
        assert!(engine.snapshot().agents.is_empty());
        // The window can tell which programs are here.
        assert!(until(|| engine.snapshot().harnesses.iter().any(|h| h.id == "shell" && h.installed)));
        assert!(engine.snapshot().harnesses.iter().any(|h| h.id == "ghost" && !h.installed));
        engine.shutdown();
    }

    #[test]
    fn a_program_updates_itself_in_a_terminal_of_its_own() {
        let shell = Arc::new(Quiet::default());
        let mut kind = shell_kind();
        // The stand-in "updates itself" by printing and ending.
        kind.update = if cfg!(windows) { vec!["/d".into(), "/c".into(), "echo updated-ok".into()] } else { vec!["-c".into(), "echo updated-ok".into()] };
        kind.args = vec![];
        let engine = start(shell, vec![kind], vec![], vec![]);
        let id = engine.run_job("shell", JobKind::Update, 80, 24).unwrap();
        assert_eq!(id, "job-update-shell");
        let job = |engine: &Handle| engine.snapshot().jobs.into_iter().find(|j| j.id == id);
        assert!(until(|| job(&engine).is_some_and(|j| !j.running && j.ok == Some(true))), "{:?}", job(&engine));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        engine
            .attach(
                &id,
                Box::new(move |bytes: &[u8]| {
                    into.lock().unwrap().extend_from_slice(bytes);
                    true
                }),
            )
            .unwrap();
        assert!(until(|| String::from_utf8_lossy(&seen.lock().unwrap()).contains("updated-ok")));
        // A job is not a desk.
        assert!(engine.snapshot().agents.is_empty());
        // It cannot be installed: it is there already, and has no package anyway.
        assert!(engine.run_job("shell", JobKind::Install, 80, 24).is_err());
        engine.forget_job(&id);
        assert!(job(&engine).is_none());
        engine.shutdown();
    }

    #[test]
    fn a_program_says_who_is_signed_in_and_signs_in_in_a_terminal_of_its_own() {
        let dir = std::env::temp_dir().join(format!("moshpit-account-{}-{}", std::process::id(), now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let said = dir.join("said.txt");
        std::fs::write(&said, "Not logged in\n").unwrap();
        // The stand-in says who is signed in by printing a file, and "signs in" by printing a word and ending.
        let asked = |line: String| if cfg!(windows) { vec!["/d".to_string(), "/c".to_string(), line] } else { vec!["-c".to_string(), line] };
        let mut kind = shell_kind();
        kind.args = vec![];
        kind.account = asked(format!("{} {}", if cfg!(windows) { "type" } else { "cat" }, said.display()));
        kind.login = asked("echo signed-in-ok".into());
        let engine = start(Arc::new(Quiet::default()), vec![kind], vec![], vec![]);
        let here = |engine: &Handle| engine.snapshot().harnesses.into_iter().find(|h| h.id == "shell").unwrap();
        // Before it has been asked, nothing is claimed either way. It is asked as the office starts.
        assert!(until(|| here(&engine).signed_in == Some(false)), "{:?}", here(&engine));
        assert_eq!(here(&engine).account, "Signed out");
        assert!(here(&engine).login_line.ends_with("echo signed-in-ok") || here(&engine).login_line.ends_with("\"echo signed-in-ok\""), "{}", here(&engine).login_line);

        // Signing in runs its own command where it can be watched, and it is asked again when that ends.
        std::fs::write(&said, "Logged in using a test\n").unwrap();
        let id = engine.run_job("shell", JobKind::SignIn, 80, 24).unwrap();
        assert_eq!(id, "job-signin-shell");
        assert!(until(|| engine.snapshot().jobs.iter().any(|j| j.id == id && j.kind == "signin" && j.ok == Some(true))), "{:?}", engine.snapshot().jobs);
        assert!(until(|| here(&engine).signed_in == Some(true)), "{:?}", here(&engine));
        assert_eq!(here(&engine).account, "Logged in using a test");

        // Asked again when someone looks. An answer that says neither leaves what was known:
        // a program that could not be asked has not said that nobody is signed in.
        std::fs::remove_file(&said).unwrap();
        engine.refresh_accounts();
        assert!(until(|| engine.inner.asking.lock().unwrap().is_empty()));
        assert_eq!(here(&engine).signed_in, Some(true));
        std::fs::write(&said, "Not logged in\n").unwrap();
        engine.refresh_accounts();
        assert!(until(|| here(&engine).signed_in == Some(false)), "{:?}", here(&engine));

        // A program that cannot say is never asked, and has no way to sign in from here.
        let plain = start(Arc::new(Quiet::default()), vec![shell_kind()], vec![], vec![]);
        plain.refresh_accounts();
        assert!(plain.inner.asking.lock().unwrap().is_empty());
        assert!(until(|| !here(&plain).version.is_empty() || here(&plain).installed));
        assert_eq!((here(&plain).signed_in, here(&plain).account.as_str(), here(&plain).login_line.as_str()), (None, "", ""));
        assert!(plain.run_job("shell", JobKind::SignIn, 80, 24).unwrap_err().contains("sign in from its terminal"));
        plain.shutdown();

        engine.forget_job(&id);
        engine.shutdown();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn desks_from_before_come_back_asleep() {
        let shell = Arc::new(Quiet::default());
        let saved = vec![
            SavedDesk {
                id: "a".into(),
                harness: "shell".into(),
                title: "Old friend".into(),
                title_locked: true,
                auto_title: String::new(),
                cwd: std::env::temp_dir().to_string_lossy().into_owned(),
                session: None,
                session_verified: false,
                created_ms: 1,
                launch: Default::default(),
            },
            // A desk whose kind of program is no longer in the table cannot be shown as anything.
            SavedDesk { id: "b".into(), harness: "gone".into(), title: "Stranger".into(), title_locked: false, auto_title: String::new(), cwd: String::new(), session: None, session_verified: false, created_ms: 2, launch: Default::default() },
        ];
        let engine = start(shell, vec![shell_kind()], saved, vec![("a".to_string(), b"said before the restart\r\n".to_vec())]);
        let agents = engine.snapshot().agents;
        assert_eq!(agents.len(), 1);
        assert_eq!((agents[0].title.as_str(), agents[0].phase, agents[0].running, agents[0].resumable), ("Old friend", Phase::Asleep, false, false));
        // What its terminal last showed is there to see, without its program being started.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        engine
            .attach(
                "a",
                Box::new(move |bytes: &[u8]| {
                    into.lock().unwrap().extend_from_slice(bytes);
                    true
                }),
            )
            .unwrap();
        assert!(String::from_utf8_lossy(&seen.lock().unwrap()).contains("said before the restart"));
        assert!(!engine.snapshot().agents[0].running);
        engine.shutdown();
    }

    #[test]
    fn a_desk_whose_program_is_not_in_the_table_is_kept_until_it_is() {
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        let desk = |id: &str, harness: &str, title: &str| SavedDesk {
            id: id.into(),
            harness: harness.into(),
            title: title.into(),
            title_locked: true,
            auto_title: String::new(),
            cwd: cwd.clone(),
            session: Some(format!("session-{id}")),
            session_verified: true,
            created_ms: 1,
            launch: Default::default(),
        };
        // "mine" is a program the user added in harnesses.json, which this time could not be read.
        let before = vec![desk("a", "shell", "Old friend"), desk("b", "mine", "Stranger"), desk("c", "shell", "Another")];
        let stranger = before[1].clone();
        let last_screen = b"what the stranger last said\r\n".to_vec();
        let titles = |engine: &Handle| engine.snapshot().agents.into_iter().map(|a| a.title).collect::<Vec<_>>();
        let ids = |desks: &[SavedDesk]| desks.iter().map(|d| d.id.clone()).collect::<Vec<_>>();

        let shell = Arc::new(Quiet::default());
        let engine = start(shell.clone(), vec![shell_kind()], before, vec![("a".to_string(), b"a\r\n".to_vec()), ("b".to_string(), last_screen.clone())]);
        // It cannot be shown, and has no terminal to open.
        assert_eq!(titles(&engine), ["Old friend", "Another"]);
        assert!(engine.attach("b", Box::new(|_: &[u8]| true)).is_err());

        // A save made while the office is open still has it, as it was and where it was.
        engine.rename("a", "Older friend");
        assert!(until(|| shell.saved.lock().unwrap().first().is_some_and(|d| d.title == "Older friend")));
        assert_eq!(ids(&shell.saved.lock().unwrap()), ["a", "b", "c"]);
        assert_eq!(shell.saved.lock().unwrap()[1], stranger);
        // With every desk that can be seen taken away, it is the one left.
        engine.dismiss("a");
        engine.dismiss("c");
        assert!(titles(&engine).is_empty());
        assert!(until(|| *shell.saved.lock().unwrap() == [stranger.clone()]));

        // Quitting saves it too, and hands back its screen, which would otherwise be deleted.
        shell.saved.lock().unwrap().clear();
        engine.shutdown();
        let saved = shell.saved.lock().unwrap().clone();
        let screens = shell.screens.lock().unwrap().clone();
        assert_eq!(saved, [stranger]);
        assert_eq!(screens, [("b".to_string(), last_screen.clone())]);

        // With its program in the table again, it is back at its desk, with what it last showed.
        let mut mine = shell_kind();
        mine.id = "mine".into();
        mine.tag = "Mine".into();
        let engine = start(Arc::new(Quiet::default()), vec![shell_kind(), mine], saved, screens);
        let agents = engine.snapshot().agents;
        assert_eq!(agents.len(), 1);
        assert_eq!((agents[0].title.as_str(), agents[0].harness_tag.as_str(), agents[0].phase), ("Stranger", "Mine", Phase::Asleep));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        engine
            .attach(
                "b",
                Box::new(move |bytes: &[u8]| {
                    into.lock().unwrap().extend_from_slice(bytes);
                    true
                }),
            )
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), last_screen);
        engine.shutdown();
    }

    #[test]
    fn a_missing_saved_folder_never_falls_back_to_home() {
        let cwd = std::env::temp_dir().join(format!("moshpit-missing-{}", new_id())).to_string_lossy().into_owned();
        let saved: SavedDesk = serde_json::from_value(serde_json::json!({"id":"lost", "harness":"shell", "title":"Lost", "cwd":cwd})).unwrap();
        let engine = start(Arc::new(Quiet::default()), vec![shell_kind()], vec![saved], vec![]);
        assert!(engine.wake("lost", 80, 24).unwrap_err().contains("folder"));
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.agents[0].cwd, cwd);
        assert!(!snapshot.agents[0].running);
        assert!(engine.inner.terms.pids().is_empty());
        engine.shutdown();
    }

    #[test]
    fn running_jobs_are_included_in_quit_and_update_warnings() {
        let mut kind = shell_kind();
        kind.update = if cfg!(windows) { vec!["/d".into(), "/q".into()] } else { vec![] };
        // Exercise the warning with a job that is still running, without relying on process timing.
        let engine = start(Arc::new(Quiet::default()), vec![kind], vec![], vec![]);
        engine.inner.state.lock().unwrap().jobs.push(Job { id: "job-update-shell".into(), harness: "shell".into(), kind: JobKind::Update, running: true, ok: None });
        assert!(engine.quit_words().unwrap().contains("A Shell update"));
        assert!(engine.quit_words().unwrap().contains("Quitting interrupts it;"));
        // A sign-in still waiting on a browser is named for what it is.
        engine.inner.state.lock().unwrap().jobs.push(Job { id: "job-signin-shell".into(), harness: "shell".into(), kind: JobKind::SignIn, running: true, ok: None });
        let words = engine.quit_words().unwrap();
        assert!(words.contains("A Shell update, A Shell sign-in") && words.contains("Quitting interrupts them;"), "{words}");
        engine.inner.state.lock().unwrap().jobs.iter_mut().for_each(|job| job.running = false);
        assert!(engine.quit_words().is_none());
        engine.shutdown();
    }

    #[test]
    fn branch_refresh_follows_checkouts_for_every_adapter() {
        let dir = std::env::temp_dir().join(format!("moshpit-branch-{}", new_id()));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/HEAD"), "ref: refs/heads/before\n").unwrap();
        let mut kind = shell_kind();
        // Claude must also refresh its saved cwd when no session file exists yet.
        kind.status = StatusFrom::ClaudeSessions;
        let engine = start(Arc::new(Quiet::default()), vec![kind], vec![], vec![]);
        let id = engine.new_agent(NewAgent { harness: "shell".into(), cwd: dir.to_string_lossy().into_owned(), prompt: String::new(), title: "Branch".into(), worktree: false, launch: Default::default() }, 80, 24).unwrap();
        assert_eq!(engine.snapshot().agents[0].branch, "before");
        std::fs::write(dir.join(".git/HEAD"), "ref: refs/heads/after\n").unwrap();
        for _ in 0..SLOW_EVERY {
            engine.look();
        }
        assert_eq!(engine.snapshot().agents[0].branch, "after");
        engine.stop(&id);
        engine.shutdown();
        assert!(until(|| std::fs::remove_dir_all(&dir).is_ok()));
    }

    #[test]
    fn ids_are_what_they_should_be() {
        let id = new_uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(id.as_bytes()[14], b'4');
        assert!(matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        assert_ne!(new_id(), new_id());
    }
}
