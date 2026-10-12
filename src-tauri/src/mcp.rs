//! Local coordination for the real CLIs. Each child gets a revocable capability;
//! its stdio MCP subprocess forwards calls to this office's loopback socket.
//!
//! What one session has for another is kept here until it is read: a task (work to be
//! done, with a status of its own) or a message (a finding, a question, an answer). For
//! both, the office records how far each got on its way: see `mailbox`. A session is told
//! of something new by whichever way its program has: pushed into it (`watch`, for a
//! program that takes pushes and a user who has turned that on), said at the next step of
//! its work by a hook (`bridge::run_hook`), said with the answer to its next call here,
//! or, failing all of those, found when it checks its inbox. Nothing is ever typed into
//! a terminal.
mod bridge;
mod launch;
mod mailbox;
mod protocol;
mod watch;

pub use bridge::{run_hook, run_stdio, run_wait};

use crate::engine::Handle;
use crate::model::{now_ms, AgentView, HarnessView, NewAgent};
use mailbox::{Delivery, Kind, Message, Waiting};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_REQUEST: usize = 256 * 1024;
const MAX_RESPONSE: usize = 512 * 1024;
const MAX_TASKS: usize = 256;
const MAX_CHILDREN: usize = 4;
/// Calls being answered at once. A connection that stays open is not one of them: a
/// session's watch must never be what keeps another session's call waiting.
const MAX_CALLS: usize = 16;
/// Connections that stay open: a watch for each session, and a wait for each idle one.
const MAX_HELD: usize = 96;
pub type PreparedLaunch = (Vec<String>, Vec<(String, String)>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub request_key: String,
    #[serde(default)]
    request: Value,
    pub from: String,
    pub to: String,
    pub prompt: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub spawned: bool,
    pub summary: String,
    pub files: Vec<String>,
    pub validation: String,
    pub created_ms: u64,
    pub updated_ms: u64,
    /// The task as it was assigned, for the session it was assigned to. Nought for one
    /// handed over at launch, or kept from before deliveries were recorded.
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub delivery: Delivery,
    /// What became of it, for the session that asked. Nought until there is a result;
    /// a new number each time one is reported.
    #[serde(default)]
    pub result_revision: u64,
    #[serde(default)]
    pub result_delivery: Delivery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus { Starting, Queued, InProgress, Completed, Blocked, Failed }

impl TaskStatus {
    fn finished(self) -> bool { matches!(self, Self::Completed | Self::Failed) }
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    tasks: Vec<Task>,
    #[serde(default)]
    messages: Vec<Message>,
    /// The last revision given out. See `mailbox::Journal::next`.
    #[serde(default)]
    revision: u64,
}

/// How something new may be brought to a session's attention, as the user has set it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Push {
    /// Claude Code is given hooks that say what is new at the next step of its work.
    pub hooks: bool,
    /// One of those hooks waits while the session is idle, and wakes it when something
    /// comes. A woken session takes a turn nobody at the keyboard asked for.
    pub wake: bool,
    /// Claude Code is started with its Channels preview, so a message is pushed into the
    /// session as it arrives. Off unless the user turns it on: it is a research preview,
    /// and Claude Code asks for confirmation in the terminal each time it starts.
    pub channels: bool,
}

impl Default for Push {
    fn default() -> Self { Self { hooks: true, wake: true, channels: false } }
}

/// What is known of one session's connection, in this run of its program.
#[derive(Clone, Debug, Default)]
struct Link {
    /// What its launch was given, and in a word why not when it was not.
    hooks: bool,
    wake: bool,
    channel: bool,
    note: String,
    /// Watches open from its MCP subprocess: more than none, and the subprocess is alive.
    live: usize,
    client: String,
    protocol: String,
    /// When one of its hooks last asked the office what is new, and which: hooks do run in it.
    hook_seen_ms: u64,
    hook_seen: String,
    /// Something pushed to it was afterwards fetched or acknowledged: pushes do reach it.
    channel_confirmed: bool,
    /// Which start of its program this is. What a watch from an earlier start leaves
    /// behind is not this one's to count.
    run: u64,
}

struct State {
    grants: HashMap<String, String>,
    journal: Journal,
    links: HashMap<String, Link>,
}

pub struct Hub {
    address: SocketAddr,
    executable: String,
    path: PathBuf,
    state: Mutex<State>,
    stopping: AtomicBool,
    connections: AtomicUsize,
    held: AtomicUsize,
    watchers: Mutex<Vec<watch::Watcher>>,
    serial: AtomicU64,
    push: Mutex<Push>,
}

fn random_id() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("Could not create a session capability: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// One of the calls that may be answered at once, given back when dropped or let go of.
struct Slot<'a>(Option<&'a AtomicUsize>);
impl Slot<'_> {
    fn release(&mut self) {
        if let Some(count) = self.0.take() { count.fetch_sub(1, Ordering::Relaxed); }
    }
}
impl Drop for Slot<'_> {
    fn drop(&mut self) { self.release(); }
}

/// What to record about the way of some items to a session.
enum Mark<'a> {
    Announced(&'a str),
    Fetched,
}

impl Hub {
    pub fn start(handle: Handle, path: PathBuf) -> Result<Arc<Self>, String> {
        let journal = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Journal>(&bytes).map_err(|e| format!("MCP task history could not be read; the original file is preserved: {e}"))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Journal::default(),
            Err(e) => return Err(format!("MCP task history could not be read: {e}")),
        };
        if journal.tasks.len() > MAX_TASKS || journal.messages.len() > mailbox::MAX_MESSAGES { return Err("MCP task history exceeds its limit; the original file is preserved.".into()); }
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| format!("The local MCP service could not start: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let hub = Arc::new(Self {
            address: listener.local_addr().map_err(|e| e.to_string())?,
            executable: std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().into_owned(),
            path,
            state: Mutex::new(State { grants: HashMap::new(), journal, links: HashMap::new() }),
            stopping: AtomicBool::new(false), connections: AtomicUsize::new(0), held: AtomicUsize::new(0),
            watchers: Mutex::new(Vec::new()), serial: AtomicU64::new(0), push: Mutex::new(Push::default()),
        });
        let serve = hub.clone();
        std::thread::spawn(move || {
            while !serve.stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if serve.connections.fetch_add(1, Ordering::Relaxed) >= MAX_CALLS {
                            serve.connections.fetch_sub(1, Ordering::Relaxed);
                            continue;
                        }
                        let hub = serve.clone();
                        let handle = handle.clone();
                        std::thread::spawn(move || {
                            let mut slot = Slot(Some(&hub.connections));
                            let _ = hub.connection(&handle, stream, &mut slot);
                        });
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
        });
        Ok(hub)
    }

    /// How something new may be brought to a session's attention from now on. A session
    /// already running keeps what it was started with.
    pub fn set_push(&self, push: Push) { *self.push.lock().unwrap() = push; }

    pub fn prepare(&self, harness: &str, desk: &str, args: Vec<String>, prefix: usize, version: &str) -> Result<PreparedLaunch, String> {
        if !launch::supported(harness) { return Ok((args, vec![])); }
        let push = *self.push.lock().unwrap();
        let plan = launch::plan(harness, version, &args[..prefix.min(args.len())], push);
        let token = self.issue(desk)?;
        let env = vec![(launch::ENDPOINT.into(), self.address.to_string()), (launch::TOKEN.into(), token), (launch::PUSH.into(), if plan.channel { "channel" } else { "none" }.into())];
        let run = self.serial.fetch_add(1, Ordering::Relaxed) + 1;
        self.state.lock().unwrap().links.insert(desk.to_string(), Link { hooks: plan.hooks, wake: plan.wake, channel: plan.channel, note: plan.note.clone(), run, ..Link::default() });
        Ok((launch::arguments(harness, &self.executable, args, prefix, &plan), env))
    }

    fn issue(&self, desk: &str) -> Result<String, String> {
        let token = random_id()?;
        {
            let mut state = self.state.lock().unwrap();
            state.grants.retain(|_, owner| owner != desk);
            state.grants.insert(token.clone(), desk.to_string());
            state.links.remove(desk);
        }
        // Whatever was held open for its last run is over.
        self.wake(desk);
        Ok(token)
    }

    pub fn revoke(&self, desk: &str) {
        {
            let mut state = self.state.lock().unwrap();
            state.grants.retain(|_, owner| owner != desk);
            state.links.remove(desk);
        }
        self.wake(desk);
    }

    /// A desk has been taken away. Whatever it had been asked to do and had not finished
    /// is over: left open, each such task would count against the session that asked,
    /// and against the office, for ever.
    pub fn abandoned(&self, desk: &str) {
        let mut told = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            let mut journal = state.journal.clone();
            for at in 0..journal.tasks.len() {
                if journal.tasks[at].to != desk || journal.tasks[at].status.finished() { continue; }
                let revision = journal.next();
                let task = &mut journal.tasks[at];
                task.status = TaskStatus::Failed;
                task.summary = "The session was removed before it reported a result.".into();
                task.updated_ms = now_ms();
                task.result_revision = revision;
                told.push(task.from.clone());
            }
            // If it cannot be saved, the tasks stay as they were on disk and in memory alike.
            if told.is_empty() || self.commit(&mut state, journal).is_err() { return; }
        }
        for asker in told { self.wake(&asker); }
    }

    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.state.lock().unwrap().grants.clear();
        self.wake_all();
    }

    fn connection(&self, handle: &Handle, stream: TcpStream, slot: &mut Slot) -> io::Result<()> {
        // BSD/macOS can inherit the listener's nonblocking mode on accept.
        // This worker uses bounded blocking reads and writes, unlike the listener.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut reader = BufReader::new(stream);
        let Some(frame) = protocol::read_frame(&mut reader, MAX_REQUEST + 4096)? else { return Ok(()) };
        let request: Value = serde_json::from_slice(&frame)?;
        let token = request["token"].as_str().unwrap_or("").to_string();
        let caller = self.state.lock().unwrap().grants.get(&token).cloned().filter(|_| !self.stopping.load(Ordering::Relaxed));
        let refused = "This Moshpit session connection is no longer authorized. Restart the session from its desk to reconnect.";
        // A connection that stays open: it gives up its place among the calls first.
        if request.get("watch").is_some() || request.get("wait").is_some() {
            let Some(desk) = caller else { return protocol::write_frame(reader.get_mut(), &json!({"error":refused})) };
            slot.release();
            if self.held.fetch_add(1, Ordering::Relaxed) >= MAX_HELD {
                self.held.fetch_sub(1, Ordering::Relaxed);
                return protocol::write_frame(reader.get_mut(), &json!({"error":"The office is holding as many connections open as it keeps."}));
            }
            let result = self.hold(handle, reader.into_inner(), &desk, &token, &request);
            self.held.fetch_sub(1, Ordering::Relaxed);
            return result;
        }
        let result = match caller {
            Some(caller) if request.get("hook").is_some() => Ok(self.hook(handle, &caller, request["hook"]["event"].as_str().unwrap_or(""))),
            Some(caller) => {
                // The MCP subprocess starts in the CLI's actual directory, which
                // may have changed when the CLI created its native worktree.
                let directory = request["cwd"].as_str().map(Path::new);
                let current = handle.snapshot().agents.into_iter().find(|a| a.id == caller);
                let moved = match (directory, current.as_ref()) {
                    (Some(directory), Some(own)) => match (project_key(directory), project_key(Path::new(&own.cwd))) {
                        (Ok(actual), Ok(original)) if actual == original => handle.agent_directory(&caller, directory),
                        _ => Err("The MCP subprocess moved outside its authorized project.".into()),
                    },
                    _ => Ok(()),
                };
                moved.and_then(|_| self.call(handle, &caller, request["name"].as_str().unwrap_or(""), &request["arguments"]))
            }
            None => Err(refused.into()),
        };
        let response = match result { Ok(value) => json!({"value":value}), Err(error) => json!({"error":error}) };
        protocol::write_frame(reader.get_mut(), &response)
    }

    fn commit(&self, state: &mut State, journal: Journal) -> Result<(), String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                let disk: Journal = serde_json::from_slice(&bytes).map_err(|_| "The task history became unreadable. The original file is preserved; repair it and restart the office.".to_string())?;
                if disk != state.journal { return Err("The task history changed outside Moshpit. Restart the office before changing tasks.".into()); }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && state.journal.tasks.is_empty() && state.journal.messages.is_empty() => {}
            Err(error) => return Err(format!("The task history could not be checked before saving: {error}")),
        }
        let bytes = serde_json::to_vec_pretty(&journal).map_err(|e| e.to_string())?;
        let temporary = self.path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|e| format!("The MCP task could not be saved: {e}"))?;
        drop(file);
        std::fs::rename(&temporary, &self.path).map_err(|e| format!("The MCP task could not be saved: {e}"))?;
        state.journal = journal;
        Ok(())
    }

    /// The session holds a capability: it was started from here and has not been stopped.
    /// That is not the same as its program having connected: see `live`.
    fn granted(&self, id: &str) -> bool { self.state.lock().unwrap().grants.values().any(|desk| desk == id) }

    /// Its MCP subprocess is running and has a watch open, as of the last second or so.
    fn live(&self, id: &str) -> bool { self.state.lock().unwrap().links.get(id).is_some_and(|link| link.live > 0) }

    /// What a session, or the window, is told about how things reach a session: each
    /// word is something the office was configured with or has seen, and no more.
    fn link_view(&self, desk: &str) -> Value {
        let state = self.state.lock().unwrap();
        let link = state.links.get(desk).cloned().unwrap_or_default();
        let seen = if link.hook_seen_ms == 0 { Value::Null } else { json!(link.hook_seen_ms) };
        json!({
            "granted": state.grants.values().any(|owner| owner == desk),
            "live": link.live > 0,
            "client": link.client,
            "protocol": link.protocol,
            // The quickest way it is told of something new, of those it was started with.
            "push": if link.channel { "channel" } else if link.hooks { "hooks" } else { "poll" },
            "channel": if !link.channel { "off" } else if link.channel_confirmed { "confirmed" } else { "requested" },
            "hooks": if !link.hooks { "off" } else if link.hook_seen_ms > 0 { "seen" } else { "configured" },
            "hook_seen_ms": seen,
            "hook_seen": link.hook_seen,
            // Whether an idle session is woken, or hears with its next prompt.
            "wake": link.wake,
            "note": link.note,
        })
    }

    /// Names for sessions, for a text another session will read: its title, or failing
    /// that the start of its id.
    fn namer(&self, handle: &Handle) -> impl Fn(&str) -> String {
        let titles: HashMap<String, String> = handle.snapshot().agents.into_iter().map(|a| (a.id, a.title)).collect();
        move |id: &str| match titles.get(id) {
            Some(title) => format!("\"{}\"", title.replace('"', "'")),
            None => format!("session {}", &id[..id.len().min(8)]),
        }
    }

    fn visible(&self, handle: &Handle, caller: &str, target: &str) -> Result<AgentView, String> {
        let snapshot = handle.snapshot();
        let own = snapshot.agents.iter().find(|a| a.id == caller).ok_or("Your desk is gone.")?;
        if !own.running { return Err("Your session is not running.".into()); }
        let own_cwd = own.cwd.clone();
        let other = snapshot.agents.into_iter().find(|a| a.id == target).ok_or("That desk is gone.")?;
        if project_key(Path::new(&own_cwd))? != project_key(Path::new(&other.cwd))? {
            return Err("That session belongs to a different project.".into());
        }
        Ok(other)
    }

    fn task(&self, caller: &str, id: &str) -> Result<Task, String> {
        let state = self.state.lock().unwrap();
        state.journal.tasks.iter().find(|t| t.id == id && (t.from == caller || t.to == caller)).cloned().ok_or_else(|| "That task is not assigned to you or requested by you.".into())
    }

    fn message(&self, caller: &str, id: &str) -> Result<Message, String> {
        let state = self.state.lock().unwrap();
        state.journal.messages.iter().find(|m| m.id == id && (m.from == caller || m.to == caller)).cloned().ok_or_else(|| "That message is not to you or from you.".into())
    }

    /// Record how far some of a session's items got, and save it. What cannot be saved is
    /// not recorded: the next reading records it again.
    fn mark(&self, desk: &str, items: &[(Kind, String, u64)], how: Mark) {
        if items.is_empty() { return; }
        let now = now_ms();
        let mut state = self.state.lock().unwrap();
        let mut journal = state.journal.clone();
        let (mut changed, mut reached) = (false, false);
        for (kind, id, revision) in items {
            let Some((current, delivery)) = journal.delivery_mut(desk, *kind, id) else { continue };
            let revision = (*revision).min(current);
            changed |= match how {
                Mark::Announced(via) => delivery.announce(revision, via, now),
                Mark::Fetched => {
                    // Read after being pushed: pushes do reach this session.
                    reached |= delivery.notified >= revision && delivery.notified_via == "channel";
                    delivery.fetch(revision, now)
                }
            };
        }
        if changed && self.commit(&mut state, journal).is_ok() && reached {
            if let Some(link) = state.links.get_mut(desk) { link.channel_confirmed = true; }
        }
    }

    /// What a session has not been told of by any way, or was pushed long enough ago,
    /// unread since, that the push cannot be taken to have arrived. Told of now, by `via`.
    /// As a turn ends (`stop`), also what it was only told of in passing and has not read.
    fn announce(&self, desk: &str, via: &str) -> Vec<Waiting> {
        let now = now_ms();
        let mut state = self.state.lock().unwrap();
        let due: Vec<Waiting> = state.journal.waiting(desk).into_iter().filter(|w| if via == "stop" { w.delivery.due_at_stop(w.revision) } else { w.delivery.due(w.revision, now) }).collect();
        if due.is_empty() { return due; }
        let mut journal = state.journal.clone();
        for item in &due {
            if let Some((_, delivery)) = journal.delivery_mut(desk, item.kind, &item.id) { delivery.tell(item.revision, via, now); }
        }
        // Not saved, not said: it is said the next time instead.
        if self.commit(&mut state, journal).is_err() { return Vec::new(); }
        due
    }

    /// A hook in a session asks what is new for it. Asking is itself evidence: hooks run there.
    fn hook(&self, handle: &Handle, caller: &str, event: &str) -> Value {
        if let Some(link) = self.state.lock().unwrap().links.get_mut(caller) { (link.hook_seen_ms, link.hook_seen) = (now_ms(), crate::harness::shorten(event, 40)); }
        // A session whose program has not connected yet has no tools to read anything with:
        // told now, it could only say so. It is told at the first step after it has.
        if !self.live(caller) { return json!({"context":null}); }
        // As a turn ends, what is said keeps the turn going: there, and only there, what
        // was mentioned and not read is said again.
        let stop = event == "Stop";
        let due = self.announce(caller, if stop { "stop" } else { "hook" });
        if due.is_empty() { return json!({"context":null}); }
        let name = self.namer(handle);
        json!({"context":if stop { mailbox::reminder(&due, &name) } else { mailbox::notice(&due, &name) }})
    }

    fn inbox(&self, handle: &Handle, caller: &str) -> Value {
        let name = self.namer(handle);
        let mut listed: Vec<(Kind, String, u64, bool)> = Vec::new();
        let value = {
            let state = self.state.lock().unwrap();
            let journal = &state.journal;
            let messages: Vec<_> = journal.messages.iter().filter(|m| m.to == caller && m.delivery.acknowledged < m.revision).take(20).map(|m| {
                let (body, whole) = mailbox::preview(&m.body, mailbox::PREVIEW);
                listed.push((Kind::Message, m.id.clone(), m.revision, whole));
                json!({"id":m.id,"from":m.from,"from_title":name(&m.from),"body":body,"truncated":!whole,"reply_to":m.reply_to,"created_ms":m.created_ms,"delivery":m.delivery.view(m.revision)})
            }).collect();
            let preview = |t: &Task| json!({"id":t.id,"from":t.from,"to":t.to,"status":t.status,"prompt":crate::harness::shorten(&t.prompt, 512),"summary":crate::harness::shorten(&t.summary, 1024),"updated_ms":t.updated_ms});
            let assigned: Vec<_> = journal.tasks.iter().filter(|t| t.to == caller && !t.status.finished()).take(20).map(|t| {
                if t.revision > 0 && t.status == TaskStatus::Queued { listed.push((Kind::Task, t.id.clone(), t.revision, t.prompt.chars().count() <= 512)); }
                let mut view = preview(t);
                view["delivery"] = t.delivery.view(t.revision);
                view
            }).collect();
            let results: Vec<_> = journal.tasks.iter().rev().filter(|t| t.from == caller && matches!(t.status, TaskStatus::Completed | TaskStatus::Blocked | TaskStatus::Failed)).take(10).map(|t| {
                if t.result_revision > 0 { listed.push((Kind::Result, t.id.clone(), t.result_revision, t.summary.chars().count() <= 1024)); }
                let mut view = preview(t);
                view["delivery"] = t.result_delivery.view(t.result_revision);
                view
            }).collect();
            // What it sent, newest first, so it can see how far each got.
            let sent: Vec<_> = journal.messages.iter().rev().filter(|m| m.from == caller).take(10).map(|m| json!({"id":m.id,"to":m.to,"to_title":name(&m.to),"body":mailbox::preview(&m.body, 160).0,"created_ms":m.created_ms,"delivery":m.delivery.view(m.revision)})).collect();
            json!({"messages":messages,"assigned_tasks":assigned,"results":results,"sent_messages":sent,"details_tool":"get_task","message_tool":"get_message","acknowledge_tool":"acknowledge","delivery":"Read through MCP; terminal input is never injected. Listing records what was shown as fetched; it accepts and acknowledges nothing."})
        };
        // A text shown whole has been handed over. One only begun has been pointed at.
        let whole: Vec<_> = listed.iter().filter(|l| l.3).map(|l| (l.0, l.1.clone(), l.2)).collect();
        let begun: Vec<_> = listed.iter().filter(|l| !l.3).map(|l| (l.0, l.1.clone(), l.2)).collect();
        self.mark(caller, &whole, Mark::Fetched);
        self.mark(caller, &begun, Mark::Announced("inbox"));
        value
    }

    fn reserve(&self, caller: &str, to: &str, args: &Value, spawned: bool) -> Result<(Task, bool), String> {
        let prompt = string(args, "prompt", 16000, false)?;
        let key = string(args, "request_key", 128, false)?;
        let task = {
            let mut state = self.state.lock().unwrap();
            if let Some(task) = state.journal.tasks.iter().find(|t| t.from == caller && t.request_key == key) {
                if task.request != *args || task.spawned != spawned || (!spawned && task.to != to) {
                    return Err("That request_key was already used for a different task.".into());
                }
                return Ok((task.clone(), false));
            }
            let active = state.journal.tasks.iter().filter(|t| t.from == caller && !t.status.finished()).count();
            if active >= MAX_CHILDREN { return Err("This session already has four unfinished delegated tasks. Finish them before delegating more. For a finding, a question or an answer, which is not work to be done, use send_message.".into()); }
            if spawned {
                if state.journal.tasks.iter().filter(|t| t.spawned && !t.status.finished()).count() >= 8 {
                    return Err("The office already has eight unfinished spawned tasks.".into());
                }
                let mut at = caller.to_string();
                let mut depth = 0;
                while let Some(parent) = state.journal.tasks.iter().find(|t| t.to == at && t.spawned) {
                    depth += 1;
                    if depth >= 3 { return Err("Delegation is limited to three levels.".into()); }
                    at = parent.from.clone();
                }
            }
            let mut journal = state.journal.clone();
            if journal.tasks.len() >= MAX_TASKS {
                let Some(oldest) = journal.tasks.iter().position(|t| t.status.finished()) else { return Err("The task history is full of unfinished tasks.".into()) };
                journal.tasks.remove(oldest);
            }
            // A task handed over at launch needs no telling: it is the first thing its session reads.
            let revision = if spawned { 0 } else { journal.next() };
            let task = Task { id: random_id()?, request_key: key, request: args.clone(), from: caller.into(), to: to.into(), prompt, spawned, status: if spawned { TaskStatus::Starting } else { TaskStatus::Queued }, summary: String::new(), files: vec![], validation: String::new(), created_ms: now_ms(), updated_ms: now_ms(), revision, delivery: Delivery::default(), result_revision: 0, result_delivery: Delivery::default() };
            journal.tasks.push(task.clone());
            self.commit(&mut state, journal)?;
            task
        };
        if !spawned { self.wake(to); }
        Ok((task, true))
    }

    /// A message for another session: saved, and its session woken to be told.
    fn post(&self, caller: &str, to: &str, args: &Value) -> Result<(Message, bool), String> {
        let body = string(args, "body", 8000, false)?;
        let key = string(args, "request_key", 128, false)?;
        let reply_to = optional_string(args, "reply_to", 128)?;
        let message = {
            let mut state = self.state.lock().unwrap();
            if let Some(message) = state.journal.messages.iter().find(|m| m.from == caller && m.request_key == key) {
                if message.request != *args || message.to != to { return Err("That request_key was already used for a different message.".into()); }
                return Ok((message.clone(), false));
            }
            if state.journal.messages.iter().filter(|m| m.from == caller && m.to == to && m.delivery.acknowledged < m.revision).count() >= mailbox::MAX_WAITING {
                return Err("That session has sixteen messages from you that it has not acknowledged. Wait for it to read them before sending more.".into());
            }
            let mut journal = state.journal.clone();
            journal.make_room()?;
            let revision = journal.next();
            let message = Message { id: random_id()?, request_key: key, request: args.clone(), from: caller.into(), to: to.into(), body, reply_to, created_ms: now_ms(), revision, delivery: Delivery::default() };
            journal.messages.push(message.clone());
            self.commit(&mut state, journal)?;
            message
        };
        self.wake(to);
        Ok((message, true))
    }

    fn update(&self, caller: &str, id: &str, status: TaskStatus, summary: String, files: Vec<String>, validation: String) -> Result<Task, String> {
        let result = {
            let mut state = self.state.lock().unwrap();
            let mut journal = state.journal.clone();
            let at = journal.tasks.iter().position(|t| t.id == id && t.to == caller).ok_or("Only the assigned session can update that task.")?;
            if journal.tasks[at].status.finished() {
                let task = &journal.tasks[at];
                if task.status == status && task.summary == summary && task.files == files && task.validation == validation { return Ok(task.clone()); }
                return Err("That task already has a final result.".into());
            }
            // A result is news for whoever asked; taking the task up is the answer to having been told of it.
            let reported = if status == TaskStatus::InProgress { 0 } else { journal.next() };
            let now = now_ms();
            let task = &mut journal.tasks[at];
            task.status = status;
            task.summary = summary;
            task.files = files;
            task.validation = validation;
            task.updated_ms = now;
            if reported > 0 { task.result_revision = reported; }
            if status == TaskStatus::InProgress && task.revision > 0 {
                task.delivery.fetch(task.revision, now);
                task.delivery.acknowledge(task.revision, now);
            }
            let result = task.clone();
            self.commit(&mut state, journal)?;
            result
        };
        if result.result_revision > 0 && status != TaskStatus::InProgress { self.wake(&result.from); }
        Ok(result)
    }

    fn launched(&self, id: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let mut journal = state.journal.clone();
        let task = journal.tasks.iter_mut().find(|t| t.id == id).ok_or("That task is gone.")?;
        if task.status == TaskStatus::Starting {
            task.status = TaskStatus::InProgress;
            task.updated_ms = now_ms();
            self.commit(&mut state, journal)?;
        }
        Ok(())
    }

    /// The session says it has seen an item: a message to it, a task assigned to it, or
    /// the result of one it asked for. Only what it was shown can be acknowledged, so an
    /// item changed since is still to be acknowledged as it is now. Said twice, it is said once.
    fn acknowledge(&self, caller: &str, id: &str, wanted: Option<u64>) -> Result<Value, String> {
        let mut state = self.state.lock().unwrap();
        let mut journal = state.journal.clone();
        let kind = journal.kind_of(caller, id).ok_or("That is not a message to you, a task assigned to you, or a task you asked for.")?;
        let Some((current, delivery)) = journal.delivery_mut(caller, kind, id) else {
            return Err(match kind { Kind::Result => "That task has no result to acknowledge yet.", _ => "That task was handed over as its session started; there is nothing to acknowledge." }.into());
        };
        let shown = delivery.shown();
        let revision = wanted.unwrap_or(shown);
        if shown == 0 { return Err("You have not been shown that yet. Read it first, with check_inbox, get_message or get_task.".into()); }
        if revision > shown { return Err(format!("Only revision {shown} of that has been shown to you. Read it again, then acknowledge it.")); }
        let pushed = delivery.notified_via == "channel";
        let changed = delivery.acknowledge(revision, now_ms());
        let view = json!({"id":id,"kind":kind.word(),"acknowledged_revision":delivery.acknowledged,"current_revision":current,"up_to_date":delivery.acknowledged >= current});
        if changed {
            self.commit(&mut state, journal)?;
            if pushed { if let Some(link) = state.links.get_mut(caller) { link.channel_confirmed = true; } }
        }
        Ok(view)
    }

    /// Whatever was asked, a session with something new waiting is told so with the answer.
    fn call(&self, handle: &Handle, caller: &str, name: &str, args: &Value) -> Result<Value, String> {
        // Schema checks also happen here: a client is not trusted to validate inputs.
        validate(name, args)?;
        let own = self.visible(handle, caller, caller)?;
        let mut value = self.answer(handle, caller, own, name, args)?;
        // The tools that list what is waiting, or read it, have said it already.
        if !matches!(name, "check_inbox" | "get_context" | "get_message" | "acknowledge") {
            let due = self.announce(caller, "hint");
            if !due.is_empty() {
                value["agent_moshpit_inbox"] = json!({"new":due.len(),"note":mailbox::notice(&due, &self.namer(handle))});
            }
        }
        Ok(value)
    }

    fn answer(&self, handle: &Handle, caller: &str, own: AgentView, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "get_context" => {
                let parent = self.state.lock().unwrap().journal.tasks.iter().find(|t| t.to == caller && t.spawned).map(|t| t.from.clone());
                Ok(json!({"session":own,"title_locked":handle.title_locked(caller),"parent_session_id":parent,"inbox":self.inbox(handle, caller),"delivery":self.link_view(caller),"programs":handle.snapshot().harnesses.into_iter().filter(|h| launch::supported(&h.id)).map(program_for_agents).collect::<Vec<_>>()}))
            }
            "rename_session" => {
                let title = crate::harness::shorten(&string(args, "title", 60, false)?, 60);
                handle.agent_rename(caller, &title)?;
                Ok(json!({"session_id":caller,"title":title}))
            }
            "set_activity" => {
                let activity = crate::harness::shorten(&string(args, "activity", 140, true)?, 140);
                handle.agent_activity(caller, &activity)?;
                Ok(json!({"session_id":caller,"activity":activity}))
            }
            "list_sessions" => {
                let mut sessions = Vec::new();
                for session in handle.snapshot().agents {
                    if self.visible(handle, caller, &session.id).is_ok() {
                        let parent = self.state.lock().unwrap().journal.tasks.iter().find(|t| t.to == session.id && t.spawned).map(|t| t.from.clone());
                        // Connected is the program's MCP subprocess being there now, not only having been started from here.
                        let (connected, link) = (self.live(&session.id), self.link_view(&session.id));
                        sessions.push(json!({"session":session_for_agents(session, caller),"mcp_connected":connected,"mcp":link,"parent_session_id":parent}));
                    }
                }
                Ok(json!({"sessions":sessions}))
            }
            "start_session" => {
                let harness = string(args, "harness", 128, false)?;
                if !launch::supported(&harness) { return Err("Delegation currently supports Claude Code and Codex.".into()); }
                let title = optional_string(args, "title", 60)?;
                let worktree = args.get("worktree").map(|v| v.as_bool().ok_or("worktree must be a boolean.")).transpose()?.unwrap_or(false);
                if handle.snapshot().agents.iter().filter(|a| a.running).count() >= 20 { return Err("The office already has twenty running sessions.".into()); }
                let launch = inherited(&own.harness, &own.launch, &harness)?;
                let child = random_id()?;
                let (task, fresh) = self.reserve(caller, &child, args, true)?;
                if !fresh { return Ok(json!({"session_id":task.to,"task_id":task.id,"task":task_view(&task),"reused":true})); }
                let prompt = format!("{}\n\nThis task was delegated through Agent Moshpit (task_id: {}). Use get_context, update your activity, and report_result with your findings, changed files and validation when finished. Do not edit files outside your assigned task.", task.prompt, task.id);
                let spec = NewAgent { harness, cwd: own.cwd, prompt, title: String::new(), worktree, launch };
                match handle.new_agent_id(spec, 100, 30, task.to.clone(), Some(&title)) {
                    Ok(_) => {
                        // A fast child may already have reported a result: do not overwrite it.
                        self.launched(&task.id)?;
                        Ok(json!({"session_id":task.to,"task_id":task.id,"reused":false}))
                    }
                    Err(error) => {
                        self.revoke(&task.to);
                        self.update(&task.to, &task.id, TaskStatus::Failed, error.clone(), vec![], String::new())?;
                        Err(error)
                    }
                }
            }
            "send_task" => {
                let target = string(args, "session_id", 128, false)?;
                let other = self.visible(handle, caller, &target)?;
                if target == caller { return Err("Choose another session for delegated work.".into()); }
                if !other.running || !self.granted(&target) { return Err("That session has no active Moshpit MCP connection. Start or resume it from Moshpit first.".into()); }
                let (task, fresh) = self.reserve(caller, &target, args, false)?;
                Ok(json!({"task":task_view(&task),"reused":!fresh,"delivery":self.sent_view(&target, &task.delivery, task.revision)}))
            }
            "send_message" => {
                let target = string(args, "session_id", 128, false)?;
                let other = self.visible(handle, caller, &target)?;
                if target == caller { return Err("Choose another session to send a message to.".into()); }
                // A session that is stopped reads it when it is carried on. One whose program is never given a connection never could.
                if !launch::supported(&other.harness) && !self.granted(&target) { return Err("That session's program has no Moshpit connection to read a message with.".into()); }
                let (message, fresh) = self.post(caller, &target, args)?;
                Ok(json!({"message":message_view(&message),"reused":!fresh,"delivery":self.sent_view(&target, &message.delivery, message.revision)}))
            }
            "get_message" => {
                let id = string(args, "message_id", 128, false)?;
                let message = self.message(caller, &id)?;
                if message.to == caller { self.mark(caller, &[(Kind::Message, id.clone(), message.revision)], Mark::Fetched); }
                let message = self.message(caller, &id)?;
                let name = self.namer(handle);
                Ok(json!({"message":message_view(&message),"from_title":name(&message.from),"to_title":name(&message.to),"acknowledge_tool":"acknowledge"}))
            }
            "acknowledge" => {
                let id = string(args, "id", 128, false)?;
                let wanted = args.get("revision").map(|v| v.as_u64().filter(|r| *r > 0).ok_or("revision must be a positive integer.")).transpose()?;
                self.acknowledge(caller, &id, wanted)
            }
            "check_inbox" => Ok(self.inbox(handle, caller)),
            "accept_task" => {
                let id = string(args, "task_id", 128, false)?;
                let task = self.update(caller, &id, TaskStatus::InProgress, String::new(), vec![], String::new())?;
                Ok(json!({"task":task_view(&task)}))
            }
            "get_task" => {
                let id = string(args, "task_id", 128, false)?;
                let wait = args.get("wait_seconds").map(|v| v.as_u64().filter(|s| *s <= 25).ok_or("wait_seconds must be an integer from 0 to 25.")).transpose()?.unwrap_or(0);
                let deadline = Instant::now() + Duration::from_secs(wait);
                loop {
                    if self.stopping.load(Ordering::Relaxed) || !self.granted(caller) { return Err("Your Moshpit connection has ended.".into()); }
                    let task = self.task(caller, &id)?;
                    let recipient_running = handle.snapshot().agents.iter().any(|a| a.id == task.to && a.running);
                    if task.status.finished() || task.status == TaskStatus::Blocked || !recipient_running || Instant::now() >= deadline {
                        // Its whole text is handed over here: to the one it was assigned to, and its result to the one who asked.
                        let mut read = Vec::new();
                        if task.to == caller && task.revision > 0 { read.push((Kind::Task, id.clone(), task.revision)); }
                        if task.from == caller && task.result_revision > 0 { read.push((Kind::Result, id.clone(), task.result_revision)); }
                        self.mark(caller, &read, Mark::Fetched);
                        return Ok(json!({"task":task_view(&self.task(caller, &id)?),"recipient_running":recipient_running}));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            "report_result" => {
                let id = string(args, "task_id", 128, false)?;
                let status = match string(args, "status", 32, false)?.as_str() { "completed" => TaskStatus::Completed, "blocked" => TaskStatus::Blocked, "failed" => TaskStatus::Failed, _ => return Err("status must be completed, blocked or failed.".into()) };
                let summary = string(args, "summary", 16000, false)?;
                let files = args.get("files").map(|v| -> Result<Vec<String>, String> {
                    let values = v.as_array().filter(|a| a.len() <= 32).ok_or("files must be an array of at most 32 paths.")?;
                    values.iter().map(|v| v.as_str().filter(|s| s.chars().count() <= 512).map(str::to_string).ok_or_else(|| "Each file path must be a string of at most 512 characters.".into())).collect()
                }).transpose()?.unwrap_or_default();
                let validation = optional_string(args, "validation", 4000)?;
                let task = self.update(caller, &id, status, summary, files, validation)?;
                handle.agent_activity(caller, match status { TaskStatus::Completed => "Delegated task completed", TaskStatus::Blocked => "Delegated task blocked", _ => "Delegated task failed" })?;
                Ok(json!({"task":task_view(&task),"result_available_to":task.from}))
            }
            "request_attention" => {
                let message = string(args, "message", 500, false)?;
                handle.agent_attention(caller, &message)?;
                Ok(json!({"session_id":caller,"notified":true}))
            }
            _ => Err("Unknown Moshpit tool.".into()),
        }
    }

    /// What a sender is told on sending: that it is saved, and how its recipient will be
    /// told. Not that it was read: that is `delivery` on the item, later.
    fn sent_view(&self, to: &str, delivery: &Delivery, revision: u64) -> Value {
        let mut view = delivery.view(revision);
        let link = self.link_view(to);
        view["how"] = link["push"].clone();
        view["recipient_live"] = link["live"].clone();
        view["note"] = json!(match link["push"].as_str() {
            // Its program is not running. What is saved for it is said when it is carried on.
            _ if link["granted"] == false => "Saved. That session's program is not running: it is told when it is carried on.",
            Some("channel") => "Saved. It is pushed into the recipient's session; until it is fetched or acknowledged, that it arrived is not known.",
            Some("hooks") if link["wake"] == true => "Saved. The recipient is told at the next step of its work, and woken if it is idle.",
            Some("hooks") => "Saved. The recipient is told at the next step of its work or its next prompt; an idle session is not woken.",
            _ => "Saved. The recipient is told with the answer to its next Moshpit call, or when it checks its inbox.",
        });
        view
    }

    /// For the window: how each session is reached, and how far each recent item got.
    pub fn overview(&self, handle: &Handle) -> Value {
        let snapshot = handle.snapshot();
        let name = |id: &str| snapshot.agents.iter().find(|a| a.id == id).map_or_else(|| "a desk that is gone".to_string(), |a| a.title.clone());
        let desks: Vec<Value> = snapshot.agents.iter().filter(|a| launch::supported(&a.harness) || self.granted(&a.id)).map(|a| {
            let waiting = self.state.lock().unwrap().journal.waiting(&a.id).iter().filter(|w| w.delivery.unread(w.revision)).count();
            json!({"id":a.id,"title":a.title,"harness":a.harness,"harness_name":a.harness_name,"running":a.running,"waiting":waiting,"link":self.link_view(&a.id)})
        }).collect();
        let state = self.state.lock().unwrap();
        let mut items: Vec<(u64, Value)> = state.journal.messages.iter().map(|m| (m.created_ms, json!({"kind":"message","id":m.id,"from":m.from,"to":m.to,"from_title":name(&m.from),"to_title":name(&m.to),"text":mailbox::preview(&m.body, 240).0,"created_ms":m.created_ms,"delivery":m.delivery.view(m.revision)}))).collect();
        items.extend(state.journal.tasks.iter().map(|t| (t.created_ms, json!({"kind":"task","id":t.id,"from":t.from,"to":t.to,"from_title":name(&t.from),"to_title":name(&t.to),"text":mailbox::preview(&t.prompt, 240).0,"created_ms":t.created_ms,"status":t.status,"spawned":t.spawned,"delivery":if t.revision > 0 { t.delivery.view(t.revision) } else { Value::Null },"result":if t.result_revision > 0 { t.result_delivery.view(t.result_revision) } else { Value::Null }}))));
        items.sort_by_key(|(created, _)| std::cmp::Reverse(*created));
        json!({"desks":desks,"items":items.into_iter().take(40).map(|(_, item)| item).collect::<Vec<_>>(),"push":*self.push.lock().unwrap()})
    }
}

/// What one session is told about a session: everything about itself, and about
/// another, everything but how it was launched. Another desk's extra instructions,
/// folders, tool rules and arguments are that desk's own business.
fn session_for_agents(view: AgentView, asking: &str) -> Value {
    let own = view.id == asking;
    let mut value = serde_json::to_value(view).expect("A desk's view contains only serializable data");
    if !own {
        value.as_object_mut().expect("A desk's view is an object").remove("launch");
    }
    value
}

/// What an agent is told about a program it could start a session of.
///
/// Every field of the view is named here, so one added to it later is decided on, not
/// handed over by default. Who is signed in is for the window: it can name a person,
/// and an agent has no use for it.
fn program_for_agents(view: HarnessView) -> Value {
    let HarnessView { id, name, tag, installed, launch, version, takes_task, worktree, latest, outdated, install_line, update_line, install_needs: _, signed_in: _, account: _, login_line: _ } = view;
    json!({"id":id,"name":name,"tag":tag,"installed":installed,"launch":launch,"version":version,"takes_task":takes_task,"worktree":worktree,"latest":latest,"outdated":outdated,"install_line":install_line,"update_line":update_line})
}

/// A delegated session never runs with looser permissions than the session that asked for it.
/// A session's boundary comes from Moshpit's launch settings, but also from its program's
/// row in the harness table and from that program's own configuration, which Moshpit cannot
/// read or translate into another program's terms. So a delegated session is always the same
/// program: started from the same row, reading the same configuration, with the asker's
/// launch settings, permissions and sandbox included.
fn inherited(parent: &str, options: &crate::launch::Options, harness: &str) -> Result<crate::launch::Options, String> {
    if parent != harness {
        return Err(format!("Delegated sessions run the same program as the session that starts them, so they keep its permission boundary. Start a {parent} session instead."));
    }
    Ok(options.clone())
}

fn task_view(task: &Task) -> Value {
    let mut value = serde_json::to_value(task).expect("A task contains only serializable data");
    let object = value.as_object_mut().unwrap();
    // The saved request is for retry validation, not part of a task's contents.
    object.remove("request");
    // How far it got, in words, in place of the bare record.
    object.insert("delivery".into(), if task.revision > 0 { task.delivery.view(task.revision) } else { Value::Null });
    object.insert("result_delivery".into(), if task.result_revision > 0 { task.result_delivery.view(task.result_revision) } else { Value::Null });
    value
}

fn message_view(message: &Message) -> Value {
    let mut value = serde_json::to_value(message).expect("A message contains only serializable data");
    let object = value.as_object_mut().unwrap();
    object.remove("request");
    object.insert("delivery".into(), message.delivery.view(message.revision));
    value
}

fn validate(name: &str, args: &Value) -> Result<(), String> {
    let definitions = protocol::tools();
    let tool = definitions.as_array().unwrap().iter().find(|t| t["name"] == name).ok_or("Unknown Moshpit tool.")?;
    let object = args.as_object().ok_or("Tool arguments must be an object.")?;
    let schema = &tool["inputSchema"];
    if object.keys().any(|key| schema["properties"].get(key).is_none()) { return Err("Unexpected tool argument.".into()); }
    if schema["required"].as_array().unwrap().iter().any(|key| !object.contains_key(key.as_str().unwrap())) { return Err("A required tool argument is missing.".into()); }
    Ok(())
}

fn string(args: &Value, key: &str, max: usize, empty: bool) -> Result<String, String> {
    let value = args[key].as_str().ok_or_else(|| format!("{key} must be a string."))?;
    if value.chars().count() > max || (!empty && value.trim().is_empty()) || value.chars().any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r') {
        return Err(format!("{key} must contain {} to {max} characters without terminal control codes.", if empty { 0 } else { 1 }));
    }
    Ok(value.trim().to_string())
}

fn optional_string(args: &Value, key: &str, max: usize) -> Result<String, String> {
    if args.get(key).is_none() { Ok(String::new()) } else { string(args, key, max, true) }
}

fn project_key(cwd: &Path) -> Result<PathBuf, String> {
    let cwd = cwd.canonicalize().map_err(|e| format!("The session's directory is unavailable: {e}"))?;
    // Resolve .git ourselves, including worktree gitfiles and their commondir;
    // no shell, command interpolation, network, or git process is needed.
    for directory in cwd.ancestors() {
        let git = directory.join(".git");
        let git = if git.is_dir() { git } else if git.is_file() {
            let text = std::fs::read_to_string(&git).map_err(|e| e.to_string())?;
            let path = text.trim().strip_prefix("gitdir: ").ok_or("Invalid worktree gitfile.")?;
            directory.join(path)
        } else { continue };
        let git = git.canonicalize().map_err(|e| e.to_string())?;
        let common = git.join("commondir");
        return if common.is_file() {
            git.join(std::fs::read_to_string(common).map_err(|e| e.to_string())?.trim()).canonicalize().map_err(|e| e.to_string())
        } else { Ok(git) };
    }
    Ok(cwd)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_delivery;
