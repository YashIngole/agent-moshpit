//! Add MCP to this launch only. No user or project configuration file is edited.
use serde_json::json;

pub const ENDPOINT: &str = "MOSHPIT_MCP_ENDPOINT";
pub const TOKEN: &str = "MOSHPIT_MCP_TOKEN";
pub fn supported(harness: &str) -> bool { matches!(harness, "claude" | "codex") }

pub fn arguments(harness: &str, executable: &str, mut args: Vec<String>, prefix: usize) -> Vec<String> {
    let mut tail = args.split_off(prefix.min(args.len()));
    let mut extra = match harness {
        "claude" => vec!["--mcp-config".into(), json!({"mcpServers":{"agent_moshpit":{"type":"stdio","command":executable,"args":["--mcp"],"env":{ENDPOINT:format!("${{{ENDPOINT}}}"),TOKEN:format!("${{{TOKEN}}}")}}}}).to_string()],
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
        assert_eq!(&claude[2..], ["--resume", "conversation"]);
        let codex = arguments("codex", exe, vec!["resume".into(), "conversation".into()], 0);
        assert!(codex[1].contains("C:\\\\Program Files"));
        assert_eq!(&codex[8..], ["resume", "conversation"]);
        let wrapped = arguments("codex", exe, vec!["custom-cli.mjs".into(), "resume".into(), "conversation".into()], 1);
        assert_eq!(wrapped[0], "custom-cli.mjs");
        assert_eq!(&wrapped[9..], ["resume", "conversation"]);
    }
}
