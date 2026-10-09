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
    fn save(&self, desks: &[SavedDesk]);
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

/// Install a program, or bring it up to date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Install,
    Update,
}

impl JobKind {
    fn word(self) -> &'static str {
        match self {
            JobKind::Install => "install",
            JobKind::Update => "update",
        }
    }
}

/// An install or an update, and how it went.
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
    /// Desks whose terminal is on screen in the window right now.
    watched: HashSet<String>,
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
    looks: u64,
}

/// Something to tell the desktop. Queued, and said by a thread that holds no lock.
enum Out {
    Snapshot(Snapshot),
    Save(Vec<SavedDesk>),
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
        focused: false,
        installed: HashMap::new(),
        latest: HashMap::new(),
        jobs: Vec::new(),
        told: None,
        saved,
        looks: 0,
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
                Out::Save(desks) => teller.save(&desks),
                Out::Notify(title, body, desk) => teller.notify(&title, &body, &desk),
            }
        }
    });
    let handle = Handle { inner: Arc::new(Inner { shell, out: Mutex::new(out), table, aside, terms, state: Mutex::new(state), home, stopping: AtomicBool::new(false) }) };
    *slot.lock().unwrap() = Some(handle.clone());
    handle.refresh_places();

    let looker = handle.clone();
    std::thread::spawn(move || {
        while !looker.inner.stopping.load(Ordering::Relaxed) {
            std::thread::sleep(LOOK_EVERY);
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
    // ── what the window asks for ───────────────────────────────────────────

    pub fn snapshot(&self) -> Snapshot {
        let state = self.inner.state.lock().unwrap();
        self.picture(&state)
    }

    /// Seat a new agent and start its program. Returns the desk's id.
    pub fn new_agent(&self, spec: NewAgent, cols: u16, rows: u16) -> Result<String, String> {
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
        let args = harness::start_args(&kind, session.as_deref(), if named { &title } else { "" }, task, spec.worktree);
        let (program, args) = harness::command_line(&found, &args)?;

        let id = new_id();
        let desk = SavedDesk { id: id.clone(), harness: kind.id.clone(), title, title_locked: named, auto_title: auto, cwd: cwd.clone(), session, created_ms: now_ms() };
        let asks = status::asks_trust(kind.trust, &self.inner.home, Path::new(&cwd));
        // Starting a program takes a moment; the lock is not held for it.
        self.inner.terms.start(&id, Launch { program, args, cwd, cols, rows })?;
        let mut state = self.inner.state.lock().unwrap();
        state.office.add(desk, now_ms());
        state.office.started(&id, !task.is_empty(), asks, now_ms());
        self.place(&mut state, &id);
        self.publish(&mut state);
        Ok(id)
    }

    /// Make sure a desk's program is running: someone asleep is woken, carrying
    /// on where they left off when their program can.
    pub fn wake(&self, id: &str, cols: u16, rows: u16) -> Result<(), String> {
        if self.inner.terms.running(id) {
            return Ok(());
        }
        let (kind, cwd, session) = {
            let state = self.inner.state.lock().unwrap();
            let desk = state.office.get(id).ok_or("That desk is gone.")?;
            (self.kind(&desk.saved.harness)?.clone(), desk.saved.cwd.clone(), desk.saved.session.clone())
        };
        let found = self.locate(&kind)?;
        // Carry on when it can; otherwise a fresh start in the same folder.
        let (args, fresh) = match harness::resume_args(&kind, session.as_deref()) {
            Some(args) => (args, None),
            None => {
                let fresh = (kind.session == SessionFrom::Given).then(new_uuid);
                (harness::start_args(&kind, fresh.as_deref(), "", "", false), fresh)
            }
        };
        let (program, args) = harness::command_line(&found, &args)?;
        let cwd = if Path::new(&cwd).is_dir() { cwd } else { String::new() };
        let asks = !cwd.is_empty() && status::asks_trust(kind.trust, &self.inner.home, Path::new(&cwd));

        let outcome = self.inner.terms.start(id, Launch { program, args, cwd, cols, rows });
        let mut state = self.inner.state.lock().unwrap();
        let now = now_ms();
        let result = match outcome {
            Ok(_) => {
                if let Some(fresh) = fresh {
                    state.office.set_session(id, &fresh);
                }
                state.office.started(id, false, asks, now);
                Ok(())
            }
            Err(why) => {
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
        };
        let Some(runner) = runner else {
            return Err(match kind {
                JobKind::Install => format!("The office does not know how to install {}. Install it the way its makers say, then it will be offered here.", program.name),
                JobKind::Update => format!("The office does not know how to update {}.", program.name),
            });
        };
        let (found, words) = match runner {
            Runner::Npm(words) => {
                let npm = harness::find("npm").ok_or_else(|| format!("{} is installed with npm, which was not found. Install Node.js from nodejs.org, then try again.", program.name))?;
                (npm, words)
            }
            Runner::Itself(words) => (self.locate(&program)?, words),
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
        self.inner.terms.start(&id, Launch { program: exe, args, cwd, cols, rows })?;
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

    pub fn resize(&self, id: &str, cols: u16, rows: u16) {
        self.inner.terms.resize(id, cols, rows);
    }

    /// End a desk's program. The desk stays, asleep.
    pub fn stop(&self, id: &str) {
        self.inner.state.lock().unwrap().office.will_stop(id);
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
        if n == 0 {
            return None;
        }
        let busy: Vec<_> = state.office.desks().iter().filter(|d| d.running && matches!(d.phase, Phase::Working | Phase::NeedsYou | Phase::Starting)).collect();
        let afresh: Vec<String> = busy
            .iter()
            .filter(|d| !self.kind(&d.saved.harness).is_ok_and(|h| harness::can_resume(h, d.saved.session.as_deref())))
            .map(|d| d.saved.title.clone())
            .collect();
        let head = if n == 1 { "1 agent is still busy. Quitting ends its program.".to_string() } else { format!("{n} agents are still busy. Quitting ends their programs.") };
        let tail = match (afresh.len(), n) {
            (0, 1) => " Its conversation is kept: Carry on, on its desk, picks it up again.".to_string(),
            (0, _) => " Their conversations are kept: Carry on, on each desk, picks them up again.".to_string(),
            (1, 1) => " Its program cannot carry on a conversation, so it starts afresh next time.".to_string(),
            (a, b) if a == b => " Their programs cannot carry on a conversation, so they start afresh next time.".to_string(),
            _ => format!(" {} will start afresh next time; the others carry on where they left off.", afresh.join(", ")),
        };
        Some(head + &tail)
    }

    /// Take the desk away, ending its program. The conversation is the program's to keep.
    pub fn dismiss(&self, id: &str) {
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

    /// Which terminals are on screen. The window says so when the user opens,
    /// closes or brings one back, so whatever had finished among them has been seen.
    pub fn watch(&self, ids: Vec<String>) {
        let mut state = self.inner.state.lock().unwrap();
        state.watched = ids.into_iter().collect();
        self.see(&mut state);
    }

    /// The window came to the front, went behind, or closed.
    pub fn attention(&self, visible: bool, focused: bool) {
        let mut state = self.inner.state.lock().unwrap();
        state.focused = visible && focused;
        if !visible {
            state.watched.clear();
        }
        if state.focused {
            self.see(&mut state);
        }
    }

    /// End every program. Called once, on the way out.
    pub fn shutdown(&self) {
        self.inner.stopping.store(true, Ordering::Relaxed);
        self.inner.terms.stop_all();
        let state = self.inner.state.lock().unwrap();
        self.inner.shell.save(&self.to_save(&state));
        // Only desks' screens: an install's terminal is not kept.
        let mut screens: Vec<(String, Vec<u8>)> = self.inner.terms.screens(SCREEN_HISTORY).into_iter().filter(|(id, _)| state.office.get(id).is_some()).collect();
        // And those of the desks held aside, as they were: a screen not handed over is deleted.
        screens.extend(self.inner.aside.iter().filter_map(|a| Some((a.desk.id.clone(), a.screen.clone()?))));
        self.inner.shell.save_screens(&screens);
    }

    // ── what happens by itself ─────────────────────────────────────────────

    fn exited(&self, id: &str, ok: bool) {
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
        let change = state.office.exited(id, ok, now_ms());
        self.tell(&state, change);
        self.publish(&mut state);
    }

    /// One look at every running program.
    fn look(&self) {
        let pids = self.inner.terms.pids();
        let now = now_ms();
        let mut state = self.inner.state.lock().unwrap();
        state.looks += 1;
        let slow = state.looks.is_multiple_of(SLOW_EVERY);
        for (id, pid) in pids {
            let Some(desk) = state.office.get(&id) else { continue };
            let Ok(kind) = self.kind(&desk.saved.harness) else { continue };
            let (cwd, started, has_session) = (desk.saved.cwd.clone(), desk.started_ms, desk.saved.session.is_some());
            let Some((mut signal, _title, printed)) = self.inner.terms.signal(&id, now) else { continue };

            if kind.status == StatusFrom::ClaudeSessions {
                if let Some(session) = status::read_claude_session(&self.inner.home, pid) {
                    // What Claude says of itself is better than what its terminal suggests,
                    // except before it has drawn anything.
                    if signal != Signal::Silent {
                        signal = status::claude_signal(&session).unwrap_or(signal);
                    }
                    state.office.suggest_title(&id, &session.name);
                    state.office.set_session(&id, &session.session_id);
                    if slow && !session.cwd.is_empty() {
                        let (repo, branch) = status::project(Path::new(&session.cwd));
                        state.office.set_place(&id, repo, branch);
                    }
                }
            }
            if slow && kind.session == SessionFrom::CodexRollouts && !has_session {
                if let Some(session) = status::codex_session(&self.inner.home, &cwd, started) {
                    state.office.set_session(&id, &session);
                }
            }
            let watched = state.focused && state.watched.contains(&id);
            // Before the look: what it printed while it was still drawing its opening screen is not news.
            state.office.heard(&id, printed, watched, now);
            let change = state.office.observe(&id, &signal, watched, now);
            self.tell(&state, change);
        }
        if slow {
            let ids: Vec<String> = state.office.desks().iter().filter(|d| d.running && d.branch.is_empty()).map(|d| d.saved.id.clone()).collect();
            for id in ids {
                self.place(&mut state, &id);
            }
        }
        self.publish(&mut state);
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
            Phase::Done => self.say(Out::Notify(format!("{} is done", change.title), "Open their terminal to see what they did.".to_string(), change.id.clone())),
            Phase::Failed => {
                let body = if change.note.is_empty() { "Something went wrong." } else { change.note.as_str() };
                self.say(Out::Notify(format!("{} hit a problem", change.title), body.to_string(), change.id.clone()));
            }
            _ => {}
        }
    }

    /// Work out which project and branch a desk's folder is.
    fn place(&self, state: &mut State, id: &str) {
        let Some(cwd) = state.office.get(id).map(|d| d.saved.cwd.clone()) else { return };
        let (repo, branch) = status::project(Path::new(&cwd));
        state.office.set_place(id, repo, branch);
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
        let picture = self.picture(state);
        let now = (picture.agents.clone(), picture.harnesses.clone(), picture.jobs.clone());
        if state.told.as_ref() != Some(&now) {
            state.told = Some(now);
            self.say(Out::Snapshot(picture));
        }
        let saved = self.to_save(state);
        if saved != state.saved {
            self.say(Out::Save(saved.clone()));
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
                    outdated: installed && harness::newer(&version, &latest),
                    version,
                    latest,
                    takes_task: h.task != harness::TaskArg::None,
                    worktree: !h.worktree.is_empty(),
                    install_line: harness::install_runner(h).map(|r| r.line(&h.program)).unwrap_or_default(),
                    update_line: harness::update_runner(h).map(|r| r.line(&h.program)).unwrap_or_default(),
                }
            })
            .collect();
        let jobs = state
            .jobs
            .iter()
            .map(|j| JobView {
                id: j.id.clone(),
                harness: j.harness.clone(),
                harness_name: self.kind(&j.harness).map_or_else(|_| j.harness.clone(), |h| h.name.clone()),
                kind: j.kind.word().to_string(),
                running: j.running,
                ok: j.ok,
            })
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
        let Ok(kind) = self.kind(harness_id) else { return };
        let found = harness::find(&kind.program);
        let version = found.as_ref().map(harness::version).unwrap_or_default();
        let mut state = self.inner.state.lock().unwrap();
        let entry = state.installed.entry(harness_id.to_string()).or_default();
        entry.found = found;
        entry.version = version;
        self.publish(&mut state);
    }

    /// Ask npm for the newest version of every installed program it publishes.
    fn check_updates(&self) {
        let Some(npm) = harness::find("npm") else { return };
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
            let mut state = self.inner.state.lock().unwrap();
            state.installed.entry(id).or_default().version = version;
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
        fn save(&self, desks: &[SavedDesk]) {
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
            package: String::new(),
            update: vec![],
            install: vec![],
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
        let spec = NewAgent { harness: "shell".into(), cwd: cwd.clone(), prompt: String::new(), title: "Probe".into(), worktree: false };
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
        engine.attach(&id, Box::new(move |bytes: &[u8]| {
            into.lock().unwrap().extend_from_slice(bytes);
            true
        }))
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
    fn an_agent_is_not_seated_without_a_folder_or_a_program() {
        let shell = Arc::new(Quiet::default());
        let mut missing = shell_kind();
        missing.id = "ghost".into();
        missing.name = "Ghost".into();
        missing.program = "no-such-program-moshpit".into();
        let engine = start(shell, vec![shell_kind(), missing], vec![], vec![]);
        let here = std::env::temp_dir().to_string_lossy().into_owned();
        let spec = |harness: &str, cwd: &str| NewAgent { harness: harness.into(), cwd: cwd.into(), prompt: String::new(), title: String::new(), worktree: false };

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
        engine.attach(&id, Box::new(move |bytes: &[u8]| {
            into.lock().unwrap().extend_from_slice(bytes);
            true
        }))
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
    fn desks_from_before_come_back_asleep() {
        let shell = Arc::new(Quiet::default());
        let saved = vec![
            SavedDesk { id: "a".into(), harness: "shell".into(), title: "Old friend".into(), title_locked: true, auto_title: String::new(), cwd: std::env::temp_dir().to_string_lossy().into_owned(), session: None, created_ms: 1 },
            // A desk whose kind of program is no longer in the table cannot be shown as anything.
            SavedDesk { id: "b".into(), harness: "gone".into(), title: "Stranger".into(), title_locked: false, auto_title: String::new(), cwd: String::new(), session: None, created_ms: 2 },
        ];
        let engine = start(shell, vec![shell_kind()], saved, vec![("a".to_string(), b"said before the restart\r\n".to_vec())]);
        let agents = engine.snapshot().agents;
        assert_eq!(agents.len(), 1);
        assert_eq!((agents[0].title.as_str(), agents[0].phase, agents[0].running, agents[0].resumable), ("Old friend", Phase::Asleep, false, false));
        // What its terminal last showed is there to see, without its program being started.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        engine
            .attach("a", Box::new(move |bytes: &[u8]| {
                into.lock().unwrap().extend_from_slice(bytes);
                true
            }))
            .unwrap();
        assert!(String::from_utf8_lossy(&seen.lock().unwrap()).contains("said before the restart"));
        assert!(!engine.snapshot().agents[0].running);
        engine.shutdown();
    }

    #[test]
    fn a_desk_whose_program_is_not_in_the_table_is_kept_until_it_is() {
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        let desk = |id: &str, harness: &str, title: &str| SavedDesk { id: id.into(), harness: harness.into(), title: title.into(), title_locked: true, auto_title: String::new(), cwd: cwd.clone(), session: Some(format!("session-{id}")), created_ms: 1 };
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
            .attach("b", Box::new(move |bytes: &[u8]| {
                into.lock().unwrap().extend_from_slice(bytes);
                true
            }))
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), last_screen);
        engine.shutdown();
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
