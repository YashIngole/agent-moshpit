//! Per-desk launch overrides. These are argv values, never a shell command.
use crate::harness::{self, Harness};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    None,
    Claude,
    Codex,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub model: String,
    pub effort: String,
    pub permission: String,
    pub sandbox: String,
    pub approval: String,
    pub profile: String,
    pub search: Option<bool>,
    pub additional_dirs: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub chrome: Option<bool>,
    pub instructions: String,
}

fn word(value: &str, name: &str) -> Result<(), String> {
    if value.len() > 256 || value.starts_with('-') || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(format!("{name} must be a single value of at most 256 characters."));
    }
    Ok(())
}

impl Options {
    pub fn validate(&self, provider: Provider) -> Result<(), String> {
        if provider == Provider::None {
            return if self == &Self::default() { Ok(()) } else { Err("This program does not offer launch settings. Use its terminal to configure it.".into()) };
        }
        word(&self.model, "Model")?;
        // Effort is intentionally not a fixed enum: providers can introduce new levels.
        word(&self.effort, "Effort")?;
        word(&self.profile, "Profile")?;
        if self.additional_dirs.len() > 32 || self.allowed_tools.len() > 64 || self.disallowed_tools.len() > 64 {
            return Err("Use at most 32 additional folders and 64 rules in each tool list.".into());
        }
        for value in self.additional_dirs.iter().chain(&self.allowed_tools).chain(&self.disallowed_tools) {
            if value.trim().is_empty() || value.len() > 4096 || value.contains(['\0', '\n', '\r']) {
                return Err("Each folder or tool rule must be one nonempty line.".into());
            }
        }
        if self.instructions.len() > 16_000 || self.instructions.contains('\0') {
            return Err("Extra instructions must be at most 16,000 characters.".into());
        }
        match provider {
            Provider::Claude => {
                if !["", "default", "manual", "acceptEdits", "auto", "plan", "dontAsk", "bypassPermissions"].contains(&self.permission.as_str()) {
                    return Err("Choose a Claude permission mode from the list.".into());
                }
                if !self.profile.is_empty() || self.search.is_some() || !self.sandbox.is_empty() || !self.approval.is_empty() {
                    return Err("Codex settings cannot be used with Claude Code.".into());
                }
            }
            Provider::Codex => {
                if !["", "workspace", "read-only", "auto", "yolo", "custom"].contains(&self.permission.as_str()) {
                    return Err("Choose a Codex permission mode from the list.".into());
                }
                if !["", "read-only", "workspace-write", "danger-full-access"].contains(&self.sandbox.as_str()) || !["", "on-request", "never"].contains(&self.approval.as_str()) {
                    return Err("Choose a supported sandbox and approval policy.".into());
                }
                if self.permission != "custom" && (!self.sandbox.is_empty() || !self.approval.is_empty()) {
                    return Err("Select Custom permissions before changing sandbox or approvals.".into());
                }
                if self.chrome.is_some() || !self.allowed_tools.is_empty() || !self.disallowed_tools.is_empty() || !self.instructions.is_empty() {
                    return Err("Claude settings cannot be used with Codex.".into());
                }
            }
            Provider::None => {}
        }
        Ok(())
    }

    pub fn args(&self, provider: Provider) -> Result<Vec<String>, String> {
        self.validate(provider)?;
        let mut args = Vec::new();
        let mut pair = |flag: &str, value: &str| {
            if !value.is_empty() { args.extend([flag.to_string(), value.to_string()]); }
        };
        pair("--model", &self.model);
        match provider {
            Provider::Claude => {
                pair("--effort", &self.effort);
                pair("--permission-mode", &self.permission);
                pair("--append-system-prompt", &self.instructions);
                if !self.allowed_tools.is_empty() { args.push("--allowedTools".into()); args.extend(self.allowed_tools.iter().cloned()); }
                if !self.disallowed_tools.is_empty() { args.push("--disallowedTools".into()); args.extend(self.disallowed_tools.iter().cloned()); }
                if let Some(on) = self.chrome { args.push(if on { "--chrome" } else { "--no-chrome" }.into()); }
            }
            Provider::Codex => {
                if !self.effort.is_empty() { pair("-c", &format!("model_reasoning_effort={}", toml_string(&self.effort))); }
                pair("--profile", &self.profile);
                match self.permission.as_str() {
                    "workspace" | "read-only" => {
                        pair("--sandbox", if self.permission == "workspace" { "workspace-write" } else { "read-only" });
                        pair("--ask-for-approval", "on-request");
                    }
                    "custom" => { pair("--sandbox", &self.sandbox); pair("--ask-for-approval", &self.approval); }
                    "auto" => args.push("--approve-for-me".into()),
                    "yolo" => args.push("--dangerously-bypass-approvals-and-sandbox".into()),
                    _ => {}
                }
                if let Some(on) = self.search {
                    args.extend(["-c".into(), format!("web_search={}", toml_string(if on { "live" } else { "disabled" }))]);
                }
            }
            Provider::None => {}
        }
        // Repeat flags, rather than a greedy variadic value swallowing the task.
        if provider == Provider::Claude && !self.additional_dirs.is_empty() {
            args.push("--add-dir".into());
            args.extend(self.additional_dirs.iter().cloned());
        } else {
            for dir in &self.additional_dirs { args.extend(["--add-dir".into(), dir.clone()]); }
        }
        Ok(args)
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

fn configured(h: &Harness, options: &Options) -> Result<Harness, String> {
    let extra = options.args(h.launch)?;
    let mut kind = h.clone();
    // Explicit selections replace the same settings in a user's harness row.
    // Keep unrelated arguments, including the office's terminal/session signals.
    let replaced = |flag: &str| -> bool {
        (!options.model.is_empty() && ["--model", "-m"].contains(&flag))
            || (!options.effort.is_empty() && flag == "--effort")
            || (!options.profile.is_empty() && ["--profile", "-p"].contains(&flag))
            || (!options.permission.is_empty() && ["--permission-mode", "--sandbox", "-s", "--ask-for-approval", "-a", "--dangerously-skip-permissions", "--dangerously-bypass-approvals-and-sandbox", "--yolo", "--approve-for-me", "--full-auto"].contains(&flag))
            || (options.chrome.is_some() && ["--chrome", "--no-chrome"].contains(&flag))
            || (options.search.is_some() && flag == "--search")
    };
    let takes_value = |flag: &str| ["--model", "-m", "--effort", "--profile", "-p", "--permission-mode", "--sandbox", "-s", "--ask-for-approval", "-a"].contains(&flag);
    kind.args.clear();
    let mut i = 0;
    while i < h.args.len() {
        let arg = &h.args[i];
        let flag = arg.split('=').next().unwrap_or(arg);
        if replaced(flag) {
            i += if !arg.contains('=') && takes_value(flag) { 2 } else { 1 };
            continue;
        }
        kind.args.push(arg.clone());
        i += 1;
    }
    kind.args.extend(extra);
    Ok(kind)
}

/// MCP flags belong after the configured harness prefix, before resume/task
/// arguments. Overrides can change that prefix's length.
pub(crate) fn prefix_len(h: &Harness, options: &Options) -> Result<usize, String> {
    Ok(configured(h, options)?.args.len())
}

pub fn start_args(h: &Harness, session: Option<&str>, title: &str, task: &str, worktree: bool, options: &Options) -> Result<Vec<String>, String> {
    let mut args = harness::start_args(&configured(h, options)?, session, title, task, worktree);
    // Stop variadic Claude flags, and keep a task beginning with '-' positional.
    if !task.trim().is_empty() && h.launch != Provider::None && matches!(h.task, harness::TaskArg::Last) {
        args.insert(args.len() - 1, "--".into());
    }
    Ok(args)
}

pub fn resume_args(h: &Harness, session: Option<&str>, options: &Options) -> Result<Option<Vec<String>>, String> {
    Ok(harness::resume_args(&configured(h, options)?, session))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn kind(provider: Provider) -> Harness { harness::built_in().into_iter().find(|h| h.launch == provider).unwrap() }

    #[test]
    fn inherit_changes_no_arguments() {
        for provider in [Provider::Claude, Provider::Codex] {
            let h = kind(provider);
            assert_eq!(start_args(&h, Some("session"), "Name", "", true, &Options::default()).unwrap(), harness::start_args(&h, Some("session"), "Name", "", true));
        }
    }

    #[test]
    fn options_precede_task_and_survive_resume() {
        let h = kind(Provider::Codex);
        let options = Options { model: "future-model".into(), effort: "future-effort".into(), permission: "yolo".into(), ..Options::default() };
        let start = start_args(&h, None, "", "Fix it", true, &options).unwrap();
        assert_eq!(start.last().unwrap(), "Fix it");
        assert!(start.contains(&"--worktree".into()));
        let resume = resume_args(&h, Some("saved-session"), &options).unwrap().unwrap();
        assert!(resume.contains(&"future-model".into()));
        assert!(resume.contains(&"--dangerously-bypass-approvals-and-sandbox".into()));
        assert!(resume.iter().position(|v| v == "--model").unwrap() < resume.iter().position(|v| v == "resume").unwrap());
        assert!(!resume.contains(&"--worktree".into()));
    }

    #[test]
    fn overrides_replace_conflicting_harness_flags() {
        let mut h = kind(Provider::Claude);
        h.args = vec!["--model=old".into(), "--dangerously-skip-permissions".into(), "--effort".into(), "low".into()];
        let options = Options { model: "new".into(), permission: "plan".into(), effort: "high".into(), ..Options::default() };
        let args = start_args(&h, None, "", "task", false, &options).unwrap();
        assert!(!args.contains(&"--model=old".into()));
        assert!(!args.contains(&"--dangerously-skip-permissions".into()));
        assert!(!args.contains(&"low".into()));
        assert!(args.contains(&"plan".into()));
    }

    #[test]
    fn invalid_or_cross_provider_settings_are_rejected() {
        assert!(Options { permission: "yolo".into(), ..Options::default() }.args(Provider::Claude).is_err());
        assert!(Options { chrome: Some(true), ..Options::default() }.args(Provider::Codex).is_err());
        assert!(Options { model: "bad\0value".into(), ..Options::default() }.args(Provider::Claude).is_err());
        assert!(Options { approval: "never".into(), permission: "yolo".into(), ..Options::default() }.args(Provider::Codex).is_err());
    }
}
