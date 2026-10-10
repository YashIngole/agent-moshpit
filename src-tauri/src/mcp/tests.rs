use super::*;
use crate::engine::{self, Shell};
use crate::harness::{self, Harness, SessionFrom, TaskArg, TrustFrom};
use crate::model::Snapshot;
use crate::office::SavedDesk;

#[derive(Default)]
struct Quiet { notices: Mutex<Vec<String>> }
impl Shell for Quiet {
    fn snapshot(&self, _: &Snapshot) {}
    fn notify(&self, _: &str, body: &str, _: &str) { self.notices.lock().unwrap().push(body.into()); }
    fn save(&self, _: u64, _: &[SavedDesk]) {}
}

// Bound live-shell fan-out on runners with a small native PTY pool. Each test
// still exercises real terminals; pure project checks below use no shell.
static FIXTURE_TERMINALS: Mutex<()> = Mutex::new(());
struct Fixture { handle: Handle, hub: Arc<Hub>, dir: PathBuf, a: String, b: String, _terminals: std::sync::MutexGuard<'static, ()> }
impl Fixture {
    fn new() -> Self {
        let terminals = FIXTURE_TERMINALS.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("moshpit-mcp-{}", random_id().unwrap()));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let mut kind: Harness = harness::built_in().into_iter().find(|h| h.id == "codex").unwrap();
        kind.id = "shell".into();
        kind.program = if cfg!(windows) { "cmd.exe" } else { "/bin/sh" }.into();
        kind.args = if cfg!(windows) { vec!["/d".into(), "/q".into()] } else { vec![] };
        kind.package.clear(); kind.update.clear(); kind.task = TaskArg::None;
        kind.trust = TrustFrom::None; kind.session = SessionFrom::Unknown; kind.resume.clear();
        let handle = engine::start(Arc::new(Quiet::default()), vec![kind], vec![], vec![]);
        let make = |cwd: &Path| NewAgent { harness: "shell".into(), cwd: cwd.to_string_lossy().into_owned(), prompt: String::new(), title: String::new(), worktree: false, launch: Default::default() };
        let a = handle.new_agent(make(&dir), 80, 24).unwrap();
        let b = handle.new_agent(make(&dir.join("sub")), 80, 24).unwrap();
        let hub = Hub::start(handle.clone(), dir.join("coordination.json")).unwrap();
        hub.issue(&a).unwrap(); hub.issue(&b).unwrap();
        Self { handle, hub, dir, a, b, _terminals: terminals }
    }
    fn call(&self, caller: &str, name: &str, args: Value) -> Result<Value, String> { self.hub.call(&self.handle, caller, name, &args) }
    fn queued(&self) -> Task {
        let result = self.call(&self.a, "send_task", json!({"session_id":self.b,"prompt":"Review the API without editing files","request_key":"review-1"})).unwrap();
        let id = result["task"]["id"].as_str().unwrap();
        self.hub.task(&self.a, id).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { self.hub.shutdown(); self.handle.shutdown(); let _ = std::fs::remove_dir_all(&self.dir); }
}

#[test]
fn user_titles_win_and_agent_descriptions_do_not_fake_status() {
    let f = Fixture::new();
    f.call(&f.a, "rename_session", json!({"title":"Review checkout tests"})).unwrap();
    assert!(!f.handle.title_locked(&f.a));
    f.handle.rename(&f.a, "My chosen name");
    assert!(f.call(&f.a, "rename_session", json!({"title":"Overwrite"})).unwrap_err().contains("user"));
    f.call(&f.a, "set_activity", json!({"activity":"Reading checkout tests"})).unwrap();
    let agent = f.handle.snapshot().agents.into_iter().find(|a| a.id == f.a).unwrap();
    assert_eq!(agent.title, "My chosen name");
    assert_eq!(agent.activity, "Reading checkout tests");
    assert!(f.call(&f.a, "rename_session", json!({"title":"Bad\u{1b}[31m"})).is_err());
    assert!(f.call(&f.a, "set_activity", json!({"activity":"work","session_id":f.b})).is_err());
}

#[test]
fn queued_tasks_are_owned_idempotent_and_survive_restart() {
    let f = Fixture::new();
    let task = f.queued();
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(f.queued().id, task.id);
    assert!(f.call(&f.a, "send_task", json!({"session_id":f.b,"prompt":"A different task","request_key":"review-1"})).is_err());
    let inbox = f.call(&f.b, "check_inbox", json!({})).unwrap();
    assert_eq!(inbox["assigned_tasks"][0]["id"], task.id);
    assert!(f.call(&f.a, "accept_task", json!({"task_id":task.id})).is_err());
    f.call(&f.b, "accept_task", json!({"task_id":task.id})).unwrap();
    assert!(f.call(&f.a, "report_result", json!({"task_id":task.id,"status":"completed","summary":"Spoofed"})).is_err());
    f.call(&f.b, "report_result", json!({"task_id":task.id,"status":"blocked","summary":"Need the expected API shape"})).unwrap();
    f.call(&f.b, "accept_task", json!({"task_id":task.id})).unwrap();
    let args = json!({"task_id":task.id,"status":"completed","summary":"Reviewed the tests","files":["tests/api.test.ts"],"validation":"npm test passed"});
    f.call(&f.b, "report_result", args.clone()).unwrap();
    f.call(&f.b, "report_result", args).unwrap();
    assert!(f.call(&f.b, "accept_task", json!({"task_id":task.id})).is_err());
    let result = f.call(&f.a, "get_task", json!({"task_id":task.id})).unwrap();
    assert_eq!(result["task"]["status"], "completed");
    assert_eq!(result["task"]["validation"], "npm test passed");
    let restored = Hub::start(f.handle.clone(), f.dir.join("coordination.json")).unwrap();
    assert_eq!(restored.task(&f.a, &task.id).unwrap().summary, "Reviewed the tests");
    assert!(!restored.connected(&f.a));
    restored.shutdown();
}

#[test]
fn inbox_delivery_never_enters_the_terminal_or_implies_completion() {
    let f = Fixture::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let bytes = seen.clone();
    f.handle.attach(&f.b, Box::new(move |chunk| { bytes.lock().unwrap().extend_from_slice(chunk); true })).unwrap();
    let task = f.queued();
    std::thread::sleep(Duration::from_millis(100));
    assert!(!String::from_utf8_lossy(&seen.lock().unwrap()).contains(&task.prompt));
    f.handle.stop(&f.b);
    let result = f.call(&f.a, "get_task", json!({"task_id":task.id})).unwrap();
    assert_eq!(result["task"]["status"], "queued");
}

#[test]
fn other_projects_and_unrelated_tasks_are_private() {
    let f = Fixture::new();
    let other = std::env::temp_dir().join(format!("moshpit-other-{}", random_id().unwrap()));
    std::fs::create_dir_all(&other).unwrap();
    let id = f.handle.new_agent(NewAgent { harness: "shell".into(), cwd: other.to_string_lossy().into_owned(), prompt: String::new(), title: String::new(), worktree: false, launch: Default::default() }, 80, 24).unwrap();
    f.hub.issue(&id).unwrap();
    assert!(f.call(&f.a, "send_task", json!({"session_id":id,"prompt":"Cross-project task","request_key":"cross"})).unwrap_err().contains("different project"));
    let task = f.queued();
    assert!(f.call(&id, "get_task", json!({"task_id":task.id})).is_err());
    assert_eq!(f.call(&f.a, "list_sessions", json!({})).unwrap()["sessions"].as_array().unwrap().len(), 2);
    f.handle.stop(&id);
    let _ = std::fs::remove_dir_all(other);
}

#[test]
fn capabilities_are_revoked_and_corrupt_history_is_preserved() {
    let f = Fixture::new();
    let token = f.hub.issue(&f.a).unwrap();
    let request = |token: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":token,"name":"get_context","arguments":{}})).unwrap();
        let bytes = protocol::read_frame(&mut reader, MAX_RESPONSE).unwrap().unwrap();
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    assert!(request(&token).get("value").is_some());
    assert!(request("wrong-token").get("error").is_some());
    f.hub.revoke(&f.a);
    assert!(request(&token).get("error").is_some());
    let path = f.dir.join("corrupt.json");
    std::fs::write(&path, b"broken history").unwrap();
    assert!(Hub::start(f.handle.clone(), path.clone()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"broken history");
    f.queued();
    let journal = f.dir.join("coordination.json");
    std::fs::write(&journal, b"externally corrupted").unwrap();
    assert!(f.call(&f.a, "send_task", json!({"session_id":f.b,"prompt":"Another review","request_key":"second"})).is_err());
    assert_eq!(std::fs::read(journal).unwrap(), b"externally corrupted");
}

#[test]
fn accepted_nonblocking_socket_waits_for_the_authenticated_request() {
    let f = Fixture::new();
    let token = f.hub.issue(&f.a).unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let (server, _) = listener.accept().unwrap();
    server.set_nonblocking(true).unwrap();
    let hub = f.hub.clone();
    let handle = f.handle.clone();
    let (finished, completion) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || { finished.send(hub.connection(&handle, server)).unwrap(); });
    assert!(matches!(completion.recv_timeout(Duration::from_millis(100)), Err(std::sync::mpsc::RecvTimeoutError::Timeout)), "The connection must wait for its request instead of closing on WouldBlock");
    let mut reader = BufReader::new(client);
    protocol::write_frame(reader.get_mut(), &json!({"token":token,"name":"get_context","arguments":{}})).unwrap();
    let frame = protocol::read_frame(&mut reader, MAX_RESPONSE).unwrap().unwrap();
    assert!(serde_json::from_slice::<Value>(&frame).unwrap().get("value").is_some());
    completion.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    worker.join().unwrap();
}

#[test]
fn a_fast_child_result_is_not_overwritten_by_launch_completion() {
    let f = Fixture::new();
    let (task, _) = f.hub.reserve(&f.a, &f.b, &json!({"prompt":"Quick review","request_key":"fast"}), true).unwrap();
    f.hub.update(&f.b, &task.id, TaskStatus::Completed, "Finished immediately".into(), vec![], "Passed".into()).unwrap();
    f.hub.launched(&task.id).unwrap();
    assert_eq!(f.hub.task(&f.a, &task.id).unwrap().status, TaskStatus::Completed);
}

#[test]
fn worktrees_and_subfolders_share_a_project_but_equal_names_do_not() {
    // Project identity is a filesystem check; it needs no terminal or live shell.
    let dir = std::env::temp_dir().join(format!("moshpit-project-{}", random_id().unwrap()));
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let worktree = dir.join("sibling");
    let git_dir = dir.join(".git/worktrees/sibling");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(worktree.join(".git"), format!("gitdir: {}\n", git_dir.display())).unwrap();
    std::fs::write(git_dir.join("commondir"), "../..\n").unwrap();
    assert_eq!(project_key(&dir).unwrap(), project_key(&worktree).unwrap());
    assert_eq!(project_key(&dir).unwrap(), project_key(&dir.join("sub")).unwrap());
    let same_name = dir.join("unrelated").join(dir.file_name().unwrap());
    std::fs::create_dir_all(same_name.join(".git")).unwrap();
    assert_ne!(project_key(&dir).unwrap(), project_key(&same_name).unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn delegated_sessions_are_the_same_program_with_the_same_settings() {
    // A pure check of what a child is launched with; it needs no terminal or live shell.
    use crate::launch::Options;
    let plan = Options { permission: "plan".into(), model: "opus".into(), disallowed_tools: vec!["Bash".into()], ..Options::default() };
    assert_eq!(inherited("claude", &plan, "claude").unwrap(), plan);
    assert!(inherited("claude", &plan, "codex").unwrap_err().contains("Start a claude session"));
    // No launch overrides is not the same as no restriction: the parent's harness row or its
    // CLI's own configuration may restrict it, and another program would not read either.
    assert!(inherited("codex", &Options::default(), "claude").is_err());
    assert!(inherited("claude", &Options::default(), "codex").is_err());
    assert_eq!(inherited("codex", &Options::default(), "codex").unwrap(), Options::default());
}

#[test]
fn another_program_is_refused_through_the_tool_even_without_launch_overrides() {
    let _terminals = FIXTURE_TERMINALS.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("moshpit-mcp-boundary-{}", random_id().unwrap()));
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    // A Codex row whose own arguments carry a sandbox restriction, run here by a plain shell
    // that ignores them. Its desk has no launch overrides at all.
    let mut row: Harness = harness::built_in().into_iter().find(|h| h.id == "codex").unwrap();
    (row.program, row.args) = if cfg!(windows) {
        ("cmd.exe".into(), ["/d", "/q", "/k", "rem", "--sandbox", "read-only"].map(String::from).into())
    } else {
        ("/bin/sh".into(), ["-c", "exec /bin/sh", "--sandbox", "read-only"].map(String::from).into())
    };
    row.package.clear(); row.update.clear(); row.task = TaskArg::None;
    row.trust = TrustFrom::None; row.session = SessionFrom::Unknown; row.resume.clear();
    let handle = engine::start(Arc::new(Quiet::default()), vec![row], vec![], vec![]);
    let parent = handle.new_agent(NewAgent { harness: "codex".into(), cwd: dir.to_string_lossy().into_owned(), prompt: String::new(), title: String::new(), worktree: false, launch: Default::default() }, 80, 24).unwrap();
    let hub = Hub::start(handle.clone(), dir.join("coordination.json")).unwrap();
    hub.issue(&parent).unwrap();
    let refused = hub.call(&handle, &parent, "start_session", &json!({"harness":"claude","prompt":"Review the API","request_key":"broader"})).unwrap_err();
    assert!(refused.contains("same program"), "{refused}");
    // Nothing was recorded or started.
    assert!(hub.state.lock().unwrap().journal.tasks.is_empty());
    assert_eq!(handle.snapshot().agents.len(), 1);
    hub.shutdown();
    handle.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}
