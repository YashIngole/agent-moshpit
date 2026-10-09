//! Read model metadata from the user's CLI. No prompts or inference requests.
//! Adapters normalize provider catalogs; model IDs and effort levels are data.
use crate::harness::{self, Found, Harness};
use crate::launch::Provider;
use crate::model::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

const TTL_MS: u64 = 15 * 60 * 1000;
const FAILURE_TTL_MS: u64 = 30 * 1000;
const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChoice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub efforts: Vec<String>,
    pub resolved: String,
    pub auto_mode: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionChoice {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub models: Vec<ModelChoice>,
    pub permissions: Vec<PermissionChoice>,
    pub features: Vec<String>,
    pub fetched_ms: u64,
    pub source: String,
    pub problem: String,
}

#[derive(Default)]
pub struct Cache {
    entries: Mutex<HashMap<String, Catalog>>,
    // Catalog probes are bounded and never run under the engine's state lock.
    // Coalesce simultaneous requests rather than creating competing CLI trees.
    probing: Mutex<()>,
}

impl Cache {
    pub fn get(&self, h: &Harness, found: &Found, cwd: &str, profile: &str, refresh: bool) -> Catalog {
        // Executable/script changes invalidate the cache after a CLI upgrade.
        let fingerprint: Vec<_> = std::iter::once(&found.program).chain(&found.before).filter_map(|p| std::fs::metadata(p).ok().map(|m| (p, m.len(), m.modified().ok()))).collect();
        let key = format!("{:?}|{}|{:?}|{:?}|{}|{}", h.launch, found.program, found.before, fingerprint, cwd, profile);
        self.load(key, refresh, || discover(h, found, cwd, profile))
    }

    fn load(&self, key: String, refresh: bool, probe: impl FnOnce() -> Catalog) -> Catalog {
        let fresh = |catalog: &Catalog| now_ms().saturating_sub(catalog.fetched_ms) < if catalog.problem.is_empty() { TTL_MS } else { FAILURE_TTL_MS };
        let before = self.entries.lock().unwrap().get(&key).cloned();
        if !refresh {
            if let Some(catalog) = before.as_ref().filter(|c| fresh(c)) { return catalog.clone(); }
        }
        let requested_at = now_ms();
        let _probe = self.probing.lock().unwrap();
        if let Some(catalog) = self.entries.lock().unwrap().get(&key).filter(|c| (refresh && c.fetched_ms >= requested_at) || (!refresh && fresh(c))).cloned() {
            return catalog;
        }
        let mut catalog = probe();
        // A network failure does not erase a usable last catalog.
        if catalog.models.is_empty() {
            if let Some(previous) = before {
                catalog.models = previous.models;
                if catalog.features.is_empty() { catalog.features = previous.features; }
                if catalog.permissions.is_empty() { catalog.permissions = previous.permissions; }
                catalog.source = if catalog.models.is_empty() { catalog.source } else { format!("{} (last available)", previous.source.trim_end_matches(" (last available)")) };
            }
        }
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= 32 && !entries.contains_key(&key) {
            if let Some(oldest) = entries.iter().min_by_key(|(_, c)| c.fetched_ms).map(|(k, _)| k.clone()) { entries.remove(&oldest); }
        }
        entries.insert(key, catalog.clone());
        catalog
    }
}

struct Probe {
    child: Child,
    tree: Option<crate::proctree::Tree>,
}

impl Drop for Probe {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL); }
        let _ = self.child.kill();
        // Windows job-object close also kills descendants, including any CLI helpers.
        self.tree.take();
        let _ = self.child.wait();
    }
}

fn capture(found: &Found, args: &[String], cwd: &str, initialize: bool) -> Result<String, String> {
    let (program, args) = harness::command_line(found, args)?;
    let mut command = Command::new(program);
    command.args(args).stdin(if initialize { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::null());
    if !cwd.is_empty() && Path::new(cwd).is_dir() { command.current_dir(cwd); }
    if let Some(path) = harness::child_path() { command.env("PATH", path); }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = command.spawn().map_err(|_| "The program could not be started to read its model catalog.".to_string())?;
    let tree = crate::proctree::Tree::new();
    if let Some(tree) = &tree { tree.add(child.id()); }
    let mut probe = Probe { child, tree };
    let stdout = probe.child.stdout.take().ok_or("The program's model catalog could not be read.")?;
    let (sender, received) = mpsc::channel::<Result<String, String>>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout.take(MAX_OUTPUT + 1));
        if initialize {
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(message) = serde_json::from_str::<Value>(&line) {
                    if message["type"] == "control_response" && message["response"]["request_id"] == "model-catalog" {
                        let _ = sender.send(Ok(line));
                        return;
                    }
                }
            }
            let _ = sender.send(Err("The CLI did not return a model catalog. Try Refresh, or enter a model ID.".into()));
        } else {
            let mut bytes = Vec::new();
            let result = reader.take(MAX_OUTPUT + 1).read_to_end(&mut bytes);
            let result = if result.is_err() || bytes.len() as u64 > MAX_OUTPUT {
                Err("The CLI model catalog was too large or unreadable.".into())
            } else { String::from_utf8(bytes).map_err(|_| "The CLI model catalog was not valid text.".into()) };
            let _ = sender.send(result);
        }
    });
    if initialize {
        let request = json!({ "type": "control_request", "request_id": "model-catalog", "request": { "subtype": "initialize" } });
        let input = probe.child.stdin.as_mut().ok_or("The CLI catalog could not be initialized.")?;
        writeln!(input, "{request}").map_err(|_| "The CLI catalog could not be initialized.".to_string())?;
    }
    let text = received.recv_timeout(Duration::from_secs(20)).map_err(|_| "Reading the model catalog timed out. Try Refresh, or enter a model ID.".to_string())??;
    if !initialize {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match probe.child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) | Err(_) => return Err("The CLI could not list models. Try Refresh, or enter a model ID.".into()),
                _ if Instant::now() >= deadline => return Err("The CLI did not finish listing models.".into()),
                _ => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
    Ok(text)
}

fn discover(h: &Harness, found: &Found, cwd: &str, profile: &str) -> Catalog {
    let mut catalog = Catalog::default();
    if h.launch == Provider::None { return catalog; }
    let help = capture(found, &["--help".into()], cwd, false).unwrap_or_default();
    catalog.features = features(&help, h.launch);
    catalog.permissions = permissions(&help, h.launch);
    let args = match h.launch {
        Provider::Codex => {
            let mut args = Vec::new();
            if !profile.is_empty() { args.extend(["--profile".into(), profile.into()]); }
            args.extend(["debug".into(), "models".into()]);
            catalog.source = "Codex model catalog".into();
            args
        }
        Provider::Claude => {
            catalog.source = "Claude Code model catalog".into();
            let mut args: Vec<String> = ["--print", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--no-session-persistence"].map(String::from).into();
            if help.contains("--safe-mode") { args.push("--safe-mode".into()); }
            else {
                args.extend(["--settings".into(), "{\"disableAllHooks\":true}".into(), "--strict-mcp-config".into(), "--mcp-config".into(), "{\"mcpServers\":{}}".into()]);
            }
            args
        }
        Provider::None => unreachable!(),
    };
    match capture(found, &args, cwd, h.launch == Provider::Claude).and_then(|text| parse(&text, h.launch)) {
        Ok(models) if !models.is_empty() => catalog.models = models,
        Ok(_) => catalog.problem = "The CLI returned no models. Enter a model ID, or try Refresh after signing in.".into(),
        Err(problem) => catalog.problem = problem,
    }
    catalog.fetched_ms = now_ms();
    catalog
}

pub fn parse(text: &str, provider: Provider) -> Result<Vec<ModelChoice>, String> {
    let value: Value = serde_json::from_str(text).map_err(|_| "The CLI returned an unreadable model catalog. You can still enter a model ID.".to_string())?;
    let entries = match provider {
        Provider::Codex => value["models"].as_array(),
        Provider::Claude => value["response"]["response"]["models"].as_array(),
        Provider::None => None,
    }.ok_or("The CLI does not expose a model catalog in this version. Update it, or enter a model ID.")?;
    let mut seen = HashSet::new();
    let mut models = Vec::new();
    for entry in entries {
        let (id_key, name_key, efforts_key) = if provider == Provider::Codex { ("slug", "display_name", "supported_reasoning_levels") } else { ("value", "displayName", "supportedEffortLevels") };
        if provider == Provider::Codex && entry["visibility"].as_str().is_some_and(|v| v != "list") { continue; }
        let Some(id) = entry[id_key].as_str().filter(|s| !s.is_empty()) else { continue };
        if !seen.insert(id.to_string()) { continue; }
        let efforts = entry[efforts_key].as_array().map(|list| list.iter().filter_map(|v| if provider == Provider::Codex { v["effort"].as_str() } else { v.as_str() }).map(String::from).collect()).unwrap_or_default();
        models.push(ModelChoice {
            id: id.to_string(), name: entry[name_key].as_str().unwrap_or(id).to_string(),
            description: entry["description"].as_str().unwrap_or("").to_string(), efforts,
            resolved: entry["resolvedModel"].as_str().unwrap_or(id).to_string(),
            auto_mode: entry["supportsAutoMode"].as_bool(),
        });
    }
    Ok(models)
}

fn features(help: &str, provider: Provider) -> Vec<String> {
    let mut features = Vec::new();
    for (flag, feature) in [("--model", "model"), ("--add-dir", "additional_dirs")] {
        if help.contains(flag) { features.push(feature.into()); }
    }
    let flags: &[(&str, &str)] = match provider {
        Provider::Codex => &[("--config", "effort"), ("--profile", "profile"), ("--search", "search")],
        Provider::Claude => &[("--effort", "effort"), ("--allowedTools", "tools"), ("--chrome", "chrome"), ("--append-system-prompt", "instructions")],
        Provider::None => &[],
    };
    for (flag, feature) in flags { if help.contains(flag) { features.push((*feature).into()); } }
    features
}

fn permissions(help: &str, provider: Provider) -> Vec<PermissionChoice> {
    let choice = |id: &str, name: &str, description: &str| PermissionChoice { id: id.into(), name: name.into(), description: description.into() };
    let mut choices = vec![choice("", "Use CLI settings", "Inherits this program's configured permission settings.")];
    match provider {
        Provider::Claude if help.contains("--permission-mode") => {
            let modes = help.split("--permission-mode").nth(1).unwrap_or("");
            let modes = modes.chars().take(700).collect::<String>();
            for (id, name, description) in [
                ("default", "Manual", "Asks you to approve actions that need permission."),
                ("acceptEdits", "Accept edits", "Accepts edits; other actions follow your permission rules."),
                ("auto", "Auto", "Reviews actions automatically. Availability depends on your model and account."),
                ("plan", "Plan", "Explores the project and prepares a plan."),
                ("dontAsk", "Don't ask", "Denies actions that would require a permission prompt."),
                ("bypassPermissions", "Bypass permissions", "Skips permission checks for this session."),
            ] {
                if modes.contains(id) || (id == "default" && modes.contains("manual")) { choices.push(choice(id, name, description)); }
            }
        }
        Provider::Codex => {
            if help.contains("--sandbox") {
                choices.push(choice("workspace", "Workspace with approvals", "Works in the workspace sandbox and can request approval."));
                choices.push(choice("read-only", "Read-only sandbox", "Starts in a read-only sandbox and can request approval."));
                choices.push(choice("custom", "Custom sandbox and approvals", "Choose the sandbox and approval policy separately under Advanced."));
            }
            if help.contains("--approve-for-me") { choices.push(choice("auto", "Automatic approval review", "Routes approvals through automatic review in the workspace sandbox.")); }
            if help.contains("--dangerously-bypass-approvals-and-sandbox") { choices.push(choice("yolo", "YOLO / full access", "Skips approvals and runs without the Codex sandbox.")); }
        }
        _ => {}
    }
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(id: &str) -> Catalog {
        Catalog { models: vec![ModelChoice { id: id.into(), name: id.into(), description: String::new(), efforts: vec![], resolved: id.into(), auto_mode: None }], fetched_ms: now_ms(), source: "fixture".into(), ..Default::default() }
    }

    #[test]
    fn cache_refresh_and_offline_fallback_preserve_a_usable_catalog() {
        let cache = Cache::default();
        assert_eq!(cache.load("cli".into(), false, || fixture("old")).models[0].id, "old");
        assert_eq!(cache.load("cli".into(), false, || panic!("fresh cache must not probe")).models[0].id, "old");
        cache.entries.lock().unwrap().get_mut("cli").unwrap().fetched_ms = 0;
        assert_eq!(cache.load("cli".into(), false, || fixture("new-release")).models[0].id, "new-release");
        // Force a refresh at a later timestamp, as a user clicking Refresh does.
        std::thread::sleep(Duration::from_millis(2));
        let failed = cache.load("cli".into(), true, || Catalog { problem: "offline".into(), fetched_ms: now_ms(), ..Default::default() });
        assert_eq!(failed.models[0].id, "new-release");
        assert_eq!(failed.problem, "offline");
        assert!(failed.source.contains("last available"));
    }

    #[test]
    fn concurrent_catalog_requests_are_coalesced_and_cache_size_is_bounded() {
        use std::sync::{Arc, Barrier, atomic::{AtomicUsize, Ordering}};
        let cache = Arc::new(Cache::default());
        let barrier = Arc::new(Barrier::new(6));
        let count = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..6).map(|_| {
            let cache = cache.clone(); let barrier = barrier.clone(); let count = count.clone();
            std::thread::spawn(move || {
                barrier.wait();
                cache.load("same-cli".into(), false, || {
                    count.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(30));
                    fixture("available")
                })
            })
        }).collect();
        for thread in threads { assert_eq!(thread.join().unwrap().models[0].id, "available"); }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        for n in 0..40 { cache.load(format!("profile-{n}"), false, || fixture("available")); }
        assert_eq!(cache.entries.lock().unwrap().len(), 32);
    }

    #[test]
    fn newly_released_models_and_efforts_need_no_app_update() {
        let text = json!({ "models": [{ "slug": "future-model-99", "display_name": "Future", "visibility": "list", "supported_reasoning_levels": [{"effort":"new-level"}] }, {"slug":"hidden", "visibility":"hidden"}, {"slug":"future-model-99"}] }).to_string();
        let models = parse(&text, Provider::Codex).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "future-model-99");
        assert_eq!(models[0].efforts, ["new-level"]);
        let text = json!({"response":{"response":{"models":[{"value":"future-alias","displayName":"Future Claude","resolvedModel":"future-id", "supportedEffortLevels":["new-effort"],"supportsAutoMode":false}]}}}).to_string();
        let models = parse(&text, Provider::Claude).unwrap();
        assert_eq!(models[0].resolved, "future-id");
        assert_eq!(models[0].auto_mode, Some(false));
        assert_eq!(models[0].efforts, ["new-effort"]);
    }

    #[test]
    fn invalid_catalogs_do_not_become_fake_models() {
        assert!(parse("error", Provider::Codex).is_err());
        assert!(parse("{}", Provider::Claude).is_err());
        assert!(parse("{\"models\":[]}", Provider::Codex).unwrap().is_empty());
    }

    #[test]
    fn permission_choices_follow_the_installed_cli() {
        let modes = permissions("--permission-mode <mode> choices: manual acceptEdits plan bypassPermissions", Provider::Claude);
        assert!(modes.iter().any(|m| m.id == "default"));
        assert!(!modes.iter().any(|m| m.id == "auto"));
        let modes = permissions("--sandbox --ask-for-approval --approve-for-me --dangerously-bypass-approvals-and-sandbox", Provider::Codex);
        assert!(modes.iter().any(|m| m.id == "auto"));
        assert!(modes.iter().any(|m| m.id == "yolo"));
    }

    #[test]
    #[ignore = "live CLI compatibility check; no user prompt is sent"]
    fn installed_clis_return_live_catalogs_without_inference() {
        for h in harness::built_in().into_iter().filter(|h| h.launch != Provider::None) {
            let Some(found) = harness::find(&h.program) else { continue };
            let catalog = discover(&h, &found, "", "");
            assert!(catalog.problem.is_empty(), "{}: {}", h.name, catalog.problem);
            assert!(!catalog.models.is_empty(), "{}", h.name);
        }
    }
}
