//! Add MCP to this launch only. No user or project configuration file is edited.
use serde_json::json;

pub const ENDPOINT: &str = "MOSHPIT_MCP_ENDPOINT";
pub const TOKEN: &str = "MOSHPIT_MCP_TOKEN";
pub fn supported(harness: &str) -> bool { matches!(harness, "claude" | "codex") }

/// The office's own tools that only say something about the session's own desk, or
/// read what it was sent: its name, what it is doing, its inbox, a task's result. A
/// session uses these without the user being asked each time. A stopped desk raising
/// its hand to ask "may I rename myself?" would be the office interrupting the user
/// about the office. Starting another session, sending one a task and asking for
/// attention are not among them: those stay the program's, and the user's, to allow.
const OWN_DESK: [&str; 8] = ["get_context", "rename_session", "set_activity", "list_sessions", "check_inbox", "accept_task", "get_task", "report_result"];

pub fn arguments(harness: &str, executable: &str, mut args: Vec<String>, prefix: usize) -> Vec<String> {
    let mut tail = args.split_off(prefix.min(args.len()));
    let mut extra = match harness {
        "claude" => {
            let mut extra = vec!["--mcp-config".into(), json!({"mcpServers":{"agent_moshpit":{"type":"stdio","command":executable,"args":["--mcp"],"env":{ENDPOINT:format!("${{{ENDPOINT}}}"),TOKEN:format!("${{{TOKEN}}}")}}}}).to_string()];
            let own = OWN_DESK.iter().map(|tool| format!("mcp__agent_moshpit__{tool}"));
            // Beside the desk's own allowed tools where it has some: one list, not two that
            // a program might read as the second replacing the first.
            match args.iter().position(|word| word == "--allowedTools" || word == "--allowed-tools") {
                Some(at) => { args.splice(at + 1..at + 1, own); }
                None => { extra.push("--allowedTools".into()); extra.extend(own); }
            }
            extra
        }
        "codex" => {
            let settings = [
                format!("mcp_servers.agent_moshpit.command={}", json!(executable)),
                "mcp_servers.agent_moshpit.args=[\"--mcp\"]".into(),
                format!("mcp_servers.agent_moshpit.env_vars=[\"{ENDPOINT}\",\"{TOKEN}\"]"),
                "mcp_servers.agent_moshpit.enabled=true".into(),
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
    #[test]
    fn native_configs_preserve_resume_and_do_not_embed_credentials() {
        let exe = "C:\\Program Files\\Moshpit\\agent-moshpit.exe";
        let claude = arguments("claude", exe, vec!["--resume".into(), "conversation".into()], 0);
        let config: serde_json::Value = serde_json::from_str(&claude[1]).unwrap();
        assert_eq!(config["mcpServers"]["agent_moshpit"]["command"], exe);
        assert_eq!(config["mcpServers"]["agent_moshpit"]["env"][TOKEN], "${MOSHPIT_MCP_TOKEN}");
        // Its own desk's tools are allowed without asking; the conversation still comes last.
        assert_eq!(claude[2], "--allowedTools");
        assert_eq!(&claude[3..3 + OWN_DESK.len()], OWN_DESK.map(|tool| format!("mcp__agent_moshpit__{tool}")));
        assert!(!claude.iter().any(|word| word.ends_with("start_session") || word.ends_with("send_task") || word.ends_with("request_attention")));
        assert_eq!(&claude[claude.len() - 2..], ["--resume", "conversation"]);
        // A desk with allowed tools of its own keeps them, in the one list.
        let mine = arguments("claude", exe, vec!["--allowedTools".into(), "Read".into(), "--resume".into(), "conversation".into()], 2);
        assert_eq!(mine.iter().filter(|word| *word == "--allowedTools").count(), 1);
        assert_eq!((mine[0].as_str(), mine[1].as_str(), mine[1 + OWN_DESK.len()].as_str()), ("--allowedTools", "mcp__agent_moshpit__get_context", "Read"));
        assert_eq!(&mine[mine.len() - 2..], ["--resume", "conversation"]);
        let codex = arguments("codex", exe, vec!["resume".into(), "conversation".into()], 0);
        assert!(codex[1].contains("C:\\\\Program Files"));
        assert_eq!(&codex[8..], ["resume", "conversation"]);
        let wrapped = arguments("codex", exe, vec!["custom-cli.mjs".into(), "resume".into(), "conversation".into()], 1);
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
            let configured = arguments("codex", "moshpit", raw, prefix);
            assert_eq!(&configured[..prefix], ["custom-cli.mjs", "--sandbox", "workspace-write", "--ask-for-approval", "on-request"]);
            assert!(configured[prefix + 1].starts_with("mcp_servers.agent_moshpit.command="));
            assert_eq!(&configured[prefix + 8..], expected_tail);
        }
    }
}
