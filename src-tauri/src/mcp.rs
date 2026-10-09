//! Local coordination for the real CLIs. Each child gets a revocable capability;
//! its stdio MCP subprocess forwards calls to this office's loopback socket.
mod launch;
mod protocol;

use crate::engine::Handle;
use crate::model::{now_ms, AgentView, NewAgent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_REQUEST: usize = 256 * 1024;
const MAX_RESPONSE: usize = 512 * 1024;
const MAX_TASKS: usize = 256;
const MAX_CHILDREN: usize = 4;
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus { Starting, Queued, InProgress, Completed, Blocked, Failed }

impl TaskStatus {
    fn finished(self) -> bool { matches!(self, Self::Completed | Self::Failed) }
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal { tasks: Vec<Task> }

struct State {
    grants: HashMap<String, String>,
    journal: Journal,
}

pub struct Hub {
    address: SocketAddr,
    executable: String,
    path: PathBuf,
    state: Mutex<State>,
    stopping: AtomicBool,
    connections: AtomicUsize,
}

fn random_id() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("Could not create a session capability: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

impl Hub {
    pub fn start(handle: Handle, path: PathBuf) -> Result<Arc<Self>, String> {
        let journal = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Journal>(&bytes).map_err(|e| format!("MCP task history could not be read; the original file is preserved: {e}"))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Journal::default(),
            Err(e) => return Err(format!("MCP task history could not be read: {e}")),
        };
        if journal.tasks.len() > MAX_TASKS { return Err("MCP task history exceeds its limit; the original file is preserved.".into()); }
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| format!("The local MCP service could not start: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let hub = Arc::new(Self {
            address: listener.local_addr().map_err(|e| e.to_string())?,
            executable: std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().into_owned(),
            path,
            state: Mutex::new(State { grants: HashMap::new(), journal }),
            stopping: AtomicBool::new(false), connections: AtomicUsize::new(0),
        });
        let serve = hub.clone();
        std::thread::spawn(move || {
            while !serve.stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if serve.connections.fetch_add(1, Ordering::Relaxed) >= 16 {
                            serve.connections.fetch_sub(1, Ordering::Relaxed);
                            continue;
                        }
                        let hub = serve.clone();
                        let handle = handle.clone();
                        std::thread::spawn(move || {
                            let _ = hub.connection(&handle, stream);
                            hub.connections.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
        });
        Ok(hub)
    }

    pub fn prepare(&self, harness: &str, desk: &str, args: Vec<String>, prefix: usize) -> Result<PreparedLaunch, String> {
        if !launch::supported(harness) { return Ok((args, vec![])); }
        let token = self.issue(desk)?;
        Ok((launch::arguments(harness, &self.executable, args, prefix), vec![(launch::ENDPOINT.into(), self.address.to_string()), (launch::TOKEN.into(), token)]))
    }

    fn issue(&self, desk: &str) -> Result<String, String> {
        let token = random_id()?;
        let mut state = self.state.lock().unwrap();
        state.grants.retain(|_, owner| owner != desk);
        state.grants.insert(token.clone(), desk.to_string());
        Ok(token)
    }

    pub fn revoke(&self, desk: &str) { self.state.lock().unwrap().grants.retain(|_, owner| owner != desk); }
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.state.lock().unwrap().grants.clear();
    }

    fn connection(&self, handle: &Handle, stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut reader = BufReader::new(stream);
        let Some(frame) = protocol::read_frame(&mut reader, MAX_REQUEST + 4096)? else { return Ok(()) };
        let request: Value = serde_json::from_slice(&frame)?;
        let token = request["token"].as_str().unwrap_or("");
        let caller = self.state.lock().unwrap().grants.get(token).cloned();
        let result = match caller {
            Some(caller) if !self.stopping.load(Ordering::Relaxed) => {
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
            _ => Err("This Moshpit session connection is no longer authorized. Restart the session from its desk to reconnect.".into()),
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
            Err(error) if error.kind() == io::ErrorKind::NotFound && state.journal.tasks.is_empty() => {}
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

    fn connected(&self, id: &str) -> bool { self.state.lock().unwrap().grants.values().any(|desk| desk == id) }

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

    fn inbox(&self, caller: &str) -> Value {
        let state = self.state.lock().unwrap();
        let preview = |t: &Task| json!({"id":t.id,"from":t.from,"to":t.to,"status":t.status,"prompt":crate::harness::shorten(&t.prompt, 512),"summary":crate::harness::shorten(&t.summary, 1024),"updated_ms":t.updated_ms});
        let assigned: Vec<_> = state.journal.tasks.iter().filter(|t| t.to == caller && !t.status.finished()).take(20).map(preview).collect();
        let results: Vec<_> = state.journal.tasks.iter().rev().filter(|t| t.from == caller && matches!(t.status, TaskStatus::Completed | TaskStatus::Blocked | TaskStatus::Failed)).take(10).map(preview).collect();
        json!({"assigned_tasks":assigned,"results":results,"details_tool":"get_task","delivery":"Read through MCP; terminal input is never injected."})
    }

    fn reserve(&self, caller: &str, to: &str, args: &Value, spawned: bool) -> Result<(Task, bool), String> {
        let prompt = string(args, "prompt", 16000, false)?;
        let key = string(args, "request_key", 128, false)?;
        let mut state = self.state.lock().unwrap();
        if let Some(task) = state.journal.tasks.iter().find(|t| t.from == caller && t.request_key == key) {
            if task.request != *args || task.spawned != spawned || (!spawned && task.to != to) {
                return Err("That request_key was already used for a different task.".into());
            }
            return Ok((task.clone(), false));
        }
        let active = state.journal.tasks.iter().filter(|t| t.from == caller && !t.status.finished()).count();
        if active >= MAX_CHILDREN { return Err("This session already has four unfinished delegated tasks. Finish them before delegating more.".into()); }
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
        let task = Task { id: random_id()?, request_key: key, request: args.clone(), from: caller.into(), to: to.into(), prompt, spawned, status: if spawned { TaskStatus::Starting } else { TaskStatus::Queued }, summary: String::new(), files: vec![], validation: String::new(), created_ms: now_ms(), updated_ms: now_ms() };
        let mut journal = state.journal.clone();
        if journal.tasks.len() >= MAX_TASKS {
            let Some(oldest) = journal.tasks.iter().position(|t| t.status.finished()) else { return Err("The task history is full of unfinished tasks.".into()) };
            journal.tasks.remove(oldest);
        }
        journal.tasks.push(task.clone());
        self.commit(&mut state, journal)?;
        Ok((task, true))
    }

    fn update(&self, caller: &str, id: &str, status: TaskStatus, summary: String, files: Vec<String>, validation: String) -> Result<Task, String> {
        let mut state = self.state.lock().unwrap();
        let mut journal = state.journal.clone();
        let task = journal.tasks.iter_mut().find(|t| t.id == id && t.to == caller).ok_or("Only the assigned session can update that task.")?;
        if task.status.finished() {
            if task.status == status && task.summary == summary && task.files == files && task.validation == validation { return Ok(task.clone()); }
            return Err("That task already has a final result.".into());
        }
        task.status = status;
        task.summary = summary;
        task.files = files;
        task.validation = validation;
        task.updated_ms = now_ms();
        let result = task.clone();
        self.commit(&mut state, journal)?;
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

    fn call(&self, handle: &Handle, caller: &str, name: &str, args: &Value) -> Result<Value, String> {
        // Schema checks also happen here: a client is not trusted to validate inputs.
        validate(name, args)?;
        let own = self.visible(handle, caller, caller)?;
        match name {
            "get_context" => {
                let parent = self.state.lock().unwrap().journal.tasks.iter().find(|t| t.to == caller && t.spawned).map(|t| t.from.clone());
                Ok(json!({"session":own,"title_locked":handle.title_locked(caller),"parent_session_id":parent,"inbox":self.inbox(caller),"programs":handle.snapshot().harnesses.into_iter().filter(|h| launch::supported(&h.id)).collect::<Vec<_>>()}))
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
                        sessions.push(json!({"session":session,"mcp_connected":self.connected(&session.id),"parent_session_id":parent}));
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
                let child = random_id()?;
                let (task, fresh) = self.reserve(caller, &child, args, true)?;
                if !fresh { return Ok(json!({"session_id":task.to,"task_id":task.id,"task":task_view(&task),"reused":true})); }
                let prompt = format!("{}\n\nThis task was delegated through Agent Moshpit (task_id: {}). Use get_context, update your activity, and report_result with your findings, changed files and validation when finished. Do not edit files outside your assigned task.", task.prompt, task.id);
                let spec = NewAgent { harness, cwd: own.cwd, prompt, title: String::new(), worktree, launch: Default::default() };
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
                if !other.running || !self.connected(&target) { return Err("That session has no active Moshpit MCP connection. Start or resume it from Moshpit first.".into()); }
                let (task, fresh) = self.reserve(caller, &target, args, false)?;
                Ok(json!({"task":task_view(&task),"reused":!fresh,"delivery":"queued_for_next_inbox_check"}))
            }
            "check_inbox" => Ok(self.inbox(caller)),
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
                    if self.stopping.load(Ordering::Relaxed) || !self.connected(caller) { return Err("Your Moshpit connection has ended.".into()); }
                    let task = self.task(caller, &id)?;
                    let recipient_running = handle.snapshot().agents.iter().any(|a| a.id == task.to && a.running);
                    if task.status.finished() || task.status == TaskStatus::Blocked || !recipient_running || Instant::now() >= deadline {
                        return Ok(json!({"task":task_view(&task),"recipient_running":recipient_running}));
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
}

fn task_view(task: &Task) -> Value {
    let mut value = serde_json::to_value(task).expect("A task contains only serializable data");
    // The saved request is for retry validation, not part of a task's contents.
    value.as_object_mut().unwrap().remove("request");
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

/// Entry point used by the CLI's MCP subprocess, before Tauri or single-instance setup.
pub fn run_stdio() -> Result<(), String> {
    let address: SocketAddr = std::env::var(launch::ENDPOINT).map_err(|_| "Start this MCP connection from an Agent Moshpit session.".to_string())?.parse().map_err(|_| "Invalid Moshpit MCP endpoint.".to_string())?;
    if !address.ip().is_loopback() { return Err("The Moshpit MCP endpoint must be local.".into()); }
    let token = std::env::var(launch::TOKEN).map_err(|_| "Missing Moshpit session capability.".to_string())?;
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut initialized = false;
    while let Some(frame) = protocol::read_frame(&mut input, MAX_REQUEST).map_err(|e| e.to_string())? {
        let response = match serde_json::from_slice(&frame) {
            Ok(request) => protocol::respond(request, &mut initialized, |name, arguments| {
                let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3)).map_err(|_| "The Moshpit office is unavailable. Start or resume your session from Moshpit.".to_string())?;
                stream.set_read_timeout(Some(Duration::from_secs(30))).map_err(|e| e.to_string())?;
                stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
                let mut reader = BufReader::new(stream);
                protocol::write_frame(reader.get_mut(), &json!({"token":token,"name":name,"arguments":arguments,"cwd":std::env::current_dir().map_err(|e| e.to_string())?})).map_err(|e| e.to_string())?;
                let reply = protocol::read_frame(&mut reader, MAX_RESPONSE).map_err(|e| e.to_string())?.ok_or("The office closed the connection.")?;
                let reply: Value = serde_json::from_slice(&reply).map_err(|e| e.to_string())?;
                match reply["error"].as_str() { Some(error) => Err(error.into()), None => reply.get("value").cloned().ok_or_else(|| "Invalid office response.".into()) }
            }),
            Err(_) => Some(protocol::error(Value::Null, -32700, "Invalid JSON")),
        };
        if let Some(response) = response { protocol::write_frame(&mut output, &response).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
