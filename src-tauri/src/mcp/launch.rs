//! Add MCP to this launch only. No user or project configuration file is edited.
use super::Push;
use serde_json::{json, Value};

pub const ENDPOINT: &str = "MOSHPIT_MCP_ENDPOINT";
pub const TOKEN: &str = "MOSHPIT_MCP_TOKEN";
/// `channel` for a session whose MCP subprocess passes pushes on to its program, `none`
/// for one that does not. Always set, so a configuration that names it always resolves.
pub const PUSH: &str = "MOSHPIT_MCP_PUSH";
/// The name the office's server goes by in each program's configuration.
pub const SERVER: &str = "agent_moshpit";
pub fn supported(harness: &str) -> bool { matches!(harness, "claude" | "codex") }

/// The office's own tools that only say something about the session's own desk, or
/// read what it was sent: its name, what it is doing, its inbox, a message, a task's
/// result, and saying that one arrived. A session uses these without the user being
/// asked each time. A stopped desk raising its hand to ask "may I rename myself?" would
/// be the office interrupting the user about the office. Starting another session,
/// sending one a task or a message and asking for attention are not among them: those
/// stay the program's, and the user's, to allow.
const OWN_DESK: [&str; 10] = ["get_context", "rename_session", "set_activity", "list_sessions", "check_inbox", "accept_task", "get_task", "report_result", "get_message", "acknowledge"];

/// The first Claude Code with everything the hooks here use: a hook started without a
/// shell (2.1.139), and one at the end of a turn that adds context (2.1.163). An event
/// name it does not know has been passed over, not refused, since 2.1.101.
const HOOKS_FROM: (u32, u32, u32) = (2, 1, 163);
/// The first Claude Code that takes pushes at all.
const CHANNELS_FROM: (u32, u32, u32) = (2, 1, 80);
/// How long Claude Code lets the hook that waits go on waiting, in seconds. After that an
/// idle session is not woken until it has taken another turn.
const WAIT_SECONDS: u64 = 6 * 60 * 60;

/// What one launch is given, of what the user allows and its program can take, and in
/// a sentence why not where it is not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Hooks that say what is new at a step of the session's work.
    pub hooks: bool,
    /// One more that waits while the session is idle, and wakes it. Only with `hooks`.
    pub wake: bool,
    /// Started with Claude Code's Channels preview, so new items are pushed into it.
    pub channel: bool,
    pub note: String,
}

/// `2.1.296 (Claude Code)` as three numbers.
fn version_of(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split_whitespace().next()?.split('.').map(|part| part.parse::<u32>());
    Some((parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?))
}

/// The settings a desk passes of its own, if any: where the word after `--settings` is,
/// and whether it is settings written out (which hooks can be added to) or a file's name.
fn own_settings(configured: &[String]) -> Option<(usize, bool)> {
    let at = configured.iter().position(|word| word == "--settings")? + 1;
    let inline = serde_json::from_str::<Value>(configured.get(at)?).is_ok_and(|value| value.is_object());
    Some((at, inline))
}

/// What this launch can be given. `configured` is the part of its words that are settings:
/// the program's own and the desk's, before the conversation and the task.
pub fn plan(harness: &str, version: &str, configured: &[String], push: Push) -> Plan {
    let mut plan = Plan::default();
    // Only Claude Code has a way to be told. Another program finds what is new with its
    // next call to the office, or when it checks its inbox.
    if harness != "claude" {
        return plan;
    }
    let version = version_of(version);
    let mut notes: Vec<&str> = Vec::new();
    if push.hooks {
        match version {
            None => notes.push("Hooks were left out: Claude Code's version was not known when this session started."),
            Some(found) if found < HOOKS_FROM => notes.push("Hooks were left out: this Claude Code is older than 2.1.163."),
            Some(_) if own_settings(configured).is_some_and(|(_, inline)| !inline) => notes.push("Hooks were left out: this desk passes a settings file of its own."),
            Some(_) => (plan.hooks, plan.wake) = (true, push.wake),
        }
    }
    if push.channels {
        match version {
            None => notes.push("Pushes were left out: Claude Code's version was not known when this session started."),
            Some(found) if found < CHANNELS_FROM => notes.push("Pushes were left out: this Claude Code is older than 2.1.80."),
            Some(_) if configured.iter().any(|word| word == "--channels" || word == "--dangerously-load-development-channels") => notes.push("Pushes were left out: this desk sets channels of its own."),
            Some(_) => plan.channel = true,
        }
    }
    plan.note = notes.join(" ");
    plan
}

/// The hooks one launch of Claude Code is given. Each starts this program again, with no
/// shell between, to ask the office what is new for the session:
/// after each batch of tool calls, with each prompt, and as a turn ends (where saying
/// something keeps the turn going, once, to read it). With `wake`, one more waits while
/// the session is idle and wakes it; it is started as the session starts and as each
/// turn ends.
fn hooks(executable: &str, wake: bool) -> Value {
    let ask = |event: &str| json!({"hooks":[{"type":"command","command":executable,"args":["--mcp-hook",event],"timeout":10}]});
    let mut all = json!({"PostToolBatch":[ask("PostToolBatch")],"UserPromptSubmit":[ask("UserPromptSubmit")],"Stop":[ask("Stop")]});
    if wake {
        let wait = || json!({"hooks":[{"type":"command","command":executable,"args":["--mcp-wait"],"asyncRewake":true,"timeout":WAIT_SECONDS}]});
        all["Stop"].as_array_mut().expect("just made").push(wait());
        all["SessionStart"] = json!([wait()]);
    }
    all
}

/// The office's hooks added to settings that may have hooks of their own: theirs stay, and run first.
fn with_hooks(settings: &mut Value, ours: Value) {
    if !settings["hooks"].is_object() {
        settings["hooks"] = json!({});
    }
    for (event, added) in ours.as_object().into_iter().flatten() {
        match settings["hooks"][event].as_array_mut() {
            Some(theirs) => theirs.extend(added.as_array().into_iter().flatten().cloned()),
            None => settings["hooks"][event] = added.clone(),
        }
    }
}

pub fn arguments(harness: &str, executable: &str, mut args: Vec<String>, prefix: usize, plan: &Plan) -> Vec<String> {
    let mut tail = args.split_off(prefix.min(args.len()));
    let mut extra = match harness {
        "claude" => {
            let mut extra = vec!["--mcp-config".into(), json!({"mcpServers":{SERVER:{"type":"stdio","command":executable,"args":["--mcp"],"env":{ENDPOINT:format!("${{{ENDPOINT}}}"),TOKEN:format!("${{{TOKEN}}}"),PUSH:format!("${{{PUSH}}}")}}}}).to_string()];
            let own = OWN_DESK.iter().map(|tool| format!("mcp__{SERVER}__{tool}"));
            // Beside the desk's own allowed tools where it has some: one list, not two that
            // a program might read as the second replacing the first.
            match args.iter().position(|word| word == "--allowedTools" || word == "--allowed-tools") {
                Some(at) => { args.splice(at + 1..at + 1, own); }
                None => { extra.push("--allowedTools".into()); extra.extend(own); }
            }
            if plan.hooks {
                // Into the settings the desk already passes, where it does: one word of settings, not two.
                match own_settings(&args) {
                    Some((at, true)) => {
                        let mut settings: Value = serde_json::from_str(&args[at]).unwrap_or_else(|_| json!({}));
                        with_hooks(&mut settings, hooks(executable, plan.wake));
                        args[at] = settings.to_string();
                    }
                    // A file's name is the desk's own, and is left as it is.
                    Some((_, false)) => {}
                    None => extra.extend(["--settings".into(), json!({"hooks":hooks(executable, plan.wake)}).to_string()]),
                }
            }
            if plan.channel {
                extra.extend(["--dangerously-load-development-channels".into(), format!("server:{SERVER}")]);
            }
            extra
        }
        "codex" => {
            let settings = [
                format!("mcp_servers.{SERVER}.command={}", json!(executable)),
                format!("mcp_servers.{SERVER}.args=[\"--mcp\"]"),
                format!("mcp_servers.{SERVER}.env_vars=[\"{ENDPOINT}\",\"{TOKEN}\"]"),
                format!("mcp_servers.{SERVER}.enabled=true"),
            ];
            settings.into_iter().flat_map(|s| ["-c".to_string(), s]).collect()
        }
        _ => Vec::new(),
    };
    args.append(&mut extra);
    args.append(&mut tail);
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = "C:\\Program Files\\Moshpit\\agent-moshpit.exe";
    fn none() -> Plan { Plan::default() }

    #[test]
    fn native_configs_preserve_resume_and_do_not_embed_credentials() {
        let claude = arguments("claude", EXE, vec!["--resume".into(), "conversation".into()], 0, &none());
        let config: serde_json::Value = serde_json::from_str(&claude[1]).unwrap();
        assert_eq!(config["mcpServers"]["agent_moshpit"]["command"], EXE);
        assert_eq!(config["mcpServers"]["agent_moshpit"]["env"][TOKEN], "${MOSHPIT_MCP_TOKEN}");
        // Its own desk's tools are allowed without asking; the conversation still comes last.
        assert_eq!(claude[2], "--allowedTools");
        assert_eq!(&claude[3..3 + OWN_DESK.len()], OWN_DESK.map(|tool| format!("mcp__agent_moshpit__{tool}")));
        assert!(!claude.iter().any(|word| word.ends_with("start_session") || word.ends_with("send_task") || word.ends_with("send_message") || word.ends_with("request_attention")));
        assert_eq!(&claude[claude.len() - 2..], ["--resume", "conversation"]);
        // With nothing asked for, nothing more is added: no settings, no hooks, no pushes.
        assert!(!claude.iter().any(|word| word == "--settings" || word.contains("channels")));
        // A desk with allowed tools of its own keeps them, in the one list.
        let mine = arguments("claude", EXE, vec!["--allowedTools".into(), "Read".into(), "--resume".into(), "conversation".into()], 2, &none());
        assert_eq!(mine.iter().filter(|word| *word == "--allowedTools").count(), 1);
        assert_eq!((mine[0].as_str(), mine[1].as_str(), mine[1 + OWN_DESK.len()].as_str()), ("--allowedTools", "mcp__agent_moshpit__get_context", "Read"));
        assert_eq!(&mine[mine.len() - 2..], ["--resume", "conversation"]);
        let codex = arguments("codex", EXE, vec!["resume".into(), "conversation".into()], 0, &none());
        assert!(codex[1].contains("C:\\\\Program Files"));
        assert_eq!(&codex[8..], ["resume", "conversation"]);
        let wrapped = arguments("codex", EXE, vec!["custom-cli.mjs".into(), "resume".into(), "conversation".into()], 1, &none());
        assert_eq!(wrapped[0], "custom-cli.mjs");
        assert_eq!(&wrapped[9..], ["resume", "conversation"]);
    }

    #[test]
    fn launch_overrides_keep_flag_values_together_before_mcp_and_resume() {
        use crate::launch::{self, Options, Provider};
        let mut h = crate::harness::built_in().into_iter().find(|h| h.launch == Provider::Codex).unwrap();
        h.args = vec!["custom-cli.mjs".into(), "--full-auto".into()];
        let options = Options { permission: "custom".into(), sandbox: "workspace-write".into(), approval: "on-request".into(), ..Default::default() };
        let prefix = launch::prefix_len(&h, &options).unwrap();
        let start = launch::start_args(&h, None, "", "task", false, &options).unwrap();
        let resume = launch::resume_args(&h, Some("conversation"), &options).unwrap().unwrap();
        for raw in [start, resume] {
            let expected_tail = raw[prefix..].to_vec();
            let configured = arguments("codex", "moshpit", raw, prefix, &none());
            assert_eq!(&configured[..prefix], ["custom-cli.mjs", "--sandbox", "workspace-write", "--ask-for-approval", "on-request"]);
            assert!(configured[prefix + 1].starts_with("mcp_servers.agent_moshpit.command="));
            assert_eq!(&configured[prefix + 8..], expected_tail);
        }
    }

    #[test]
    fn what_a_launch_is_given_depends_on_the_user_the_program_and_its_version() {
        let on = Push { hooks: true, wake: true, channels: true };
        let current = plan("claude", "2.1.296 (Claude Code)", &[], on);
        assert_eq!((current.hooks, current.wake, current.channel, current.note.as_str()), (true, true, true, ""));
        // Hooks are the fallback, on unless turned off; pushes are a preview, off unless turned on.
        let usual = plan("claude", "2.1.296 (Claude Code)", &[], Push::default());
        assert_eq!((usual.hooks, usual.wake, usual.channel), (true, true, false));
        let off = plan("claude", "2.1.296", &[], Push { hooks: false, wake: true, channels: false });
        // Waking is one of the hooks: without them there is none to wake with.
        assert_eq!((off.hooks, off.wake, off.channel, off.note.as_str()), (false, false, false, ""));
        let asleep = plan("claude", "2.1.296", &[], Push { hooks: true, wake: false, channels: false });
        assert_eq!((asleep.hooks, asleep.wake), (true, false));
        // A Claude Code too old for a hook started without a shell is given none: started
        // through a shell, this program's name alone would open the office.
        let old = plan("claude", "2.1.150 (Claude Code)", &[], on);
        assert_eq!((old.hooks, old.channel), (false, true));
        assert!(old.note.contains("older than 2.1.163"), "{}", old.note);
        let older = plan("claude", "2.1.60", &[], on);
        assert!(!older.hooks && !older.channel && older.note.contains("2.1.80"), "{}", older.note);
        let unknown = plan("claude", "", &[], on);
        assert!(!unknown.hooks && !unknown.channel && unknown.note.contains("not known"));
        // A desk's own settings file cannot be added to, and its own channels are its own.
        let file = plan("claude", "2.1.296", &["--settings".into(), "C:/work/settings.json".into()], on);
        assert!(!file.hooks && file.channel && file.note.contains("settings file of its own"), "{}", file.note);
        let inline = plan("claude", "2.1.296", &["--settings".into(), "{\"enabledPlugins\":{}}".into()], on);
        assert!(inline.hooks);
        let theirs = plan("claude", "2.1.296", &["--channels".into(), "plugin:telegram@claude-plugins-official".into()], on);
        assert!(theirs.hooks && !theirs.channel && theirs.note.contains("channels of its own"));
        // Codex has neither: nothing was left out of its launch, so there is nothing to explain.
        assert_eq!(plan("codex", "0.161.0", &[], on), Plan::default());
        assert_eq!(plan("shell", "", &[], on), Plan::default());
    }

    #[test]
    fn hooks_and_pushes_are_added_for_this_launch_alone() {
        let both = Plan { hooks: true, wake: true, channel: true, note: String::new() };
        let args = arguments("claude", EXE, vec!["--resume".into(), "conversation".into()], 0, &both);
        let at = |word: &str| args.iter().position(|w| w == word).unwrap();
        let settings: Value = serde_json::from_str(&args[at("--settings") + 1]).unwrap();
        // This program, started without a shell, once for each moment it is asked at.
        for event in ["PostToolBatch", "UserPromptSubmit", "Stop"] {
            let asked = settings["hooks"][event][0]["hooks"][0].clone();
            assert_eq!(asked, json!({"type":"command","command":EXE,"args":["--mcp-hook",event],"timeout":10}), "{event}");
        }
        // The one that waits is started with the session and as each turn ends, in the background.
        for event in ["SessionStart", "Stop"] {
            let waits = settings["hooks"][event].as_array().unwrap().last().unwrap()["hooks"][0].clone();
            assert_eq!((waits["args"].clone(), waits["asyncRewake"].clone(), waits["command"].as_str()), (json!(["--mcp-wait"]), json!(true), Some(EXE)), "{event}");
        }
        assert_eq!(args[at("--dangerously-load-development-channels") + 1], "server:agent_moshpit");
        // The subprocess is told by its environment whether to pass pushes on; with none set, it is not.
        let config: Value = serde_json::from_str(&args[at("--mcp-config") + 1]).unwrap();
        assert_eq!(config["mcpServers"]["agent_moshpit"]["env"][PUSH], "${MOSHPIT_MCP_PUSH}");
        // Never in place of a settings file the desk names, even if asked to add hooks.
        let named = arguments("claude", EXE, vec!["--settings".into(), "C:/work/settings.json".into()], 2, &both);
        assert_eq!((named.iter().filter(|w| *w == "--settings").count(), named[1].as_str()), (1, "C:/work/settings.json"));
        assert_eq!(&args[args.len() - 2..], ["--resume", "conversation"]);

        // A desk that already passes settings keeps them, and its own hooks run first.
        let own = json!({"enabledPlugins":{"one@market":false},"hooks":{"Stop":[{"hooks":[{"type":"command","command":"say done"}]}]}}).to_string();
        let merged = arguments("claude", EXE, vec!["--settings".into(), own, "--verbose".into(), "--resume".into(), "c".into()], 3, &Plan { hooks: true, ..Plan::default() });
        assert_eq!(merged.iter().filter(|w| *w == "--settings").count(), 1);
        let settings: Value = serde_json::from_str(&merged[1]).unwrap();
        assert_eq!(settings["enabledPlugins"], json!({"one@market":false}));
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"][0]["command"], "say done");
        assert_eq!(settings["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert!(settings["hooks"]["PostToolBatch"].is_array());
        assert!(!merged.iter().any(|w| w.contains("channels")));
        // Told not to wake, the session is given nothing that waits.
        assert!(!settings.to_string().contains("--mcp-wait") && settings["hooks"].get("SessionStart").is_none());
        // Codex is given neither, whatever is asked.
        assert!(!arguments("codex", EXE, vec![], 0, &both).iter().any(|w| w.contains("hook") || w.contains("channel")));
    }

    #[test]
    fn a_version_is_read_from_what_the_program_prints() {
        assert_eq!(version_of("2.1.296 (Claude Code)"), Some((2, 1, 296)));
        assert_eq!(version_of("2.1.80"), Some((2, 1, 80)));
        assert_eq!(version_of(""), None);
        assert_eq!(version_of("claude version unknown"), None);
        assert!((2, 1, 163) > (2, 1, 99) && (2, 2, 0) > (2, 1, 999));
    }
}
