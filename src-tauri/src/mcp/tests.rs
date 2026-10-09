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

struct Fixture { handle: Handle, hub: Arc<Hub>, dir: PathBuf, a: String, b: String }
impl Fixture {
    fn new() -> Self {
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
        Self { handle, hub, dir, a, b }
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
fn a_fast_child_result_is_not_overwritten_by_launch_completion() {
    let f = Fixture::new();
    let (task, _) = f.hub.reserve(&f.a, &f.b, &json!({"prompt":"Quick review","request_key":"fast"}), true).unwrap();
    f.hub.update(&f.b, &task.id, TaskStatus::Completed, "Finished immediately".into(), vec![], "Passed".into()).unwrap();
    f.hub.launched(&task.id).unwrap();
    assert_eq!(f.hub.task(&f.a, &task.id).unwrap().status, TaskStatus::Completed);
}

#[test]
fn worktrees_and_subfolders_share_a_project_but_equal_names_do_not() {
    let f = Fixture::new();
    let worktree = f.dir.join("sibling");
    let git_dir = f.dir.join(".git/worktrees/sibling");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(worktree.join(".git"), format!("gitdir: {}\n", git_dir.display())).unwrap();
    std::fs::write(git_dir.join("commondir"), "../..\n").unwrap();
    assert_eq!(project_key(&f.dir).unwrap(), project_key(&worktree).unwrap());
    assert_eq!(project_key(&f.dir).unwrap(), project_key(&f.dir.join("sub")).unwrap());
}
