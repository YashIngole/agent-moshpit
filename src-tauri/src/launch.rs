//! Per-desk launch overrides. These are argv values, never a shell command.
use crate::harness::{self, Harness};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Plugins switched on or off for this desk alone, by the name the program gives each.
    /// Only those changed from the program's own settings are kept here.
    pub plugins: BTreeMap<String, bool>,
    /// Further words for the program, exactly as typed, after everything above. For
    /// whatever it has that the office has no setting for.
    pub extra_args: Vec<String>,
}

/// Words that say which conversation Claude Code carries on, or that make it answer once
/// and end. The office says the first itself, and a desk's program stays in its terminal.
const CLAUDE_OWN: [&str; 8] = ["--session-id", "--resume", "-r", "--continue", "-c", "--fork-session", "--print", "-p"];

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
        if self.plugins.len() > 64 || self.plugins.keys().any(|id| id.is_empty() || id.len() > 200 || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | '/'))) {
            return Err("A plugin is named with letters, digits and . _ - @ / only, and at most 64 can be chosen.".into());
        }
        if self.extra_args.len() > 64 || self.extra_args.iter().any(|word| word.is_empty() || word.len() > 4096 || word.chars().any(char::is_control)) {
            return Err("Use at most 64 extra arguments, each on one line and no longer than 4,096 characters.".into());
        }
        // What follows a flag may be its value; what comes first can only be a flag. A bare
        // word there would be read as a command of the program's, or as the task.
        if self.extra_args.first().is_some_and(|word| !word.starts_with('-')) {
            return Err("Extra arguments start with a flag, such as --verbose. The task goes in the box above.".into());
        }
        match provider {
            Provider::Claude => {
                if !["", "default", "manual", "acceptEdits", "auto", "plan", "dontAsk", "bypassPermissions"].contains(&self.permission.as_str()) {
                    return Err("Choose a Claude permission mode from the list.".into());
                }
                if !self.profile.is_empty() || self.search.is_some() || !self.sandbox.is_empty() || !self.approval.is_empty() {
                    return Err("Codex settings cannot be used with Claude Code.".into());
                }
                if let Some(own) = self.extra_args.iter().find(|word| CLAUDE_OWN.contains(&word.split('=').next().unwrap_or(word))) {
                    return Err(format!("{own} cannot be an extra argument. The office says which conversation a desk carries on, and its program stays in its terminal."));
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
                if self.chrome.is_some() || !self.allowed_tools.is_empty() || !self.disallowed_tools.is_empty() || !self.instructions.is_empty() || !self.plugins.is_empty() {
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
        // For this one run, on top of the program's own settings, which stay as they are.
        if provider == Provider::Claude && !self.plugins.is_empty() {
            args.extend(["--settings".into(), serde_json::json!({ "enabledPlugins": self.plugins }).to_string()]);
        }
        // Repeat flags, rather than a greedy variadic value swallowing the task.
        if provider == Provider::Claude && !self.additional_dirs.is_empty() {
            args.push("--add-dir".into());
            args.extend(self.additional_dirs.iter().cloned());
        } else {
            for dir in &self.additional_dirs { args.extend(["--add-dir".into(), dir.clone()]); }
        }
        // Last, so they are the final say where they repeat a setting above.
        args.extend(self.extra_args.iter().cloned());
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
    fn plugins_and_extra_arguments_are_passed_as_words_before_the_conversation_and_the_task() {
        let h = kind(Provider::Claude);
        let options = Options {
            plugins: BTreeMap::from([("off-one@market".to_string(), false), ("on-one@market".to_string(), true)]),
            extra_args: vec!["--remote-control".into(), "--fallback-model".into(), "a model, with a space & more".into()],
            additional_dirs: vec!["C:/other".into()],
            ..Options::default()
        };
        let start = start_args(&h, Some("session"), "Name", "Fix it", false, &options).unwrap();
        let at = |word: &str| start.iter().position(|w| w == word).unwrap_or_else(|| panic!("{word} in {start:?}"));
        // One word of settings, which is JSON naming each plugin and nothing else.
        let settings: serde_json::Value = serde_json::from_str(&start[at("--settings") + 1]).unwrap();
        assert_eq!(settings, serde_json::json!({ "enabledPlugins": { "off-one@market": false, "on-one@market": true } }));
        // The extra words come last of the settings, each exactly as it was given, and the task is still the task.
        assert_eq!(&start[at("--remote-control")..at("--remote-control") + 3], ["--remote-control", "--fallback-model", "a model, with a space & more"]);
        assert!(at("--add-dir") < at("--remote-control") && at("--remote-control") < at("--session-id"));
        assert_eq!(&start[start.len() - 2..], ["--", "Fix it"]);
        // Carried on, they are passed again, before the word that says which conversation.
        let resume = resume_args(&h, Some("session"), &options).unwrap().unwrap();
        assert!(resume.contains(&"--settings".to_string()) && resume.iter().position(|w| w == "--remote-control").unwrap() < resume.iter().position(|w| w == "--resume").unwrap());
        // The office's MCP link goes after all of them.
        assert_eq!(prefix_len(&h, &options).unwrap(), resume.iter().position(|w| w == "--resume").unwrap());

        // Codex takes extra words too, before its `resume`.
        let codex = kind(Provider::Codex);
        let extra = Options { extra_args: vec!["--enable".into(), "some_feature".into(), "-c".into(), "model_verbosity=\"low\"".into()], ..Options::default() };
        let resume = resume_args(&codex, Some("session"), &extra).unwrap().unwrap();
        assert!(resume.iter().position(|w| w == "--enable").unwrap() < resume.iter().position(|w| w == "resume").unwrap());
        // With nothing chosen, nothing is added.
        assert!(!start_args(&h, None, "", "", false, &Options::default()).unwrap().contains(&"--settings".to_string()));
    }

    #[test]
    fn extra_arguments_that_would_break_a_desk_are_refused_with_the_reason() {
        let extra = |words: &[&str]| Options { extra_args: words.iter().map(|w| w.to_string()).collect(), ..Options::default() };
        // Which conversation a desk carries on is the office's to say, and its program stays in its terminal.
        for own in ["--resume", "-r", "--continue", "-c", "--session-id", "--session-id=abc", "--fork-session", "--print", "-p"] {
            let why = extra(&["--verbose", own]).args(Provider::Claude).unwrap_err();
            assert!(why.contains(own) && why.contains("conversation"), "{own}: {why}");
        }
        // Codex's -c is a setting, not a conversation.
        assert!(extra(&["-c", "model_verbosity=low"]).args(Provider::Codex).is_ok());
        // A bare word first would be a command, or the task.
        assert!(extra(&["exec", "do it"]).args(Provider::Codex).unwrap_err().contains("start with a flag"));
        assert!(extra(&["resume"]).args(Provider::Claude).is_err());
        // One line each, and not without end.
        assert!(extra(&["--x", "two\nlines"]).args(Provider::Claude).is_err());
        assert!(extra(&["--x", ""]).args(Provider::Claude).is_err());
        assert!(Options { extra_args: vec!["--x".into(); 65], ..Options::default() }.args(Provider::Claude).is_err());
        // A plugin is named plainly, and only Claude Code has them here.
        let plugin = |id: &str| Options { plugins: BTreeMap::from([(id.to_string(), false)]), ..Options::default() };
        assert!(plugin("name@market-place_1.0").args(Provider::Claude).is_ok());
        for bad in ["", "two words", "quote\"d", "semi;colon", "brace}"] {
            assert!(plugin(bad).args(Provider::Claude).is_err(), "{bad:?}");
        }
        assert!(plugin("name@market").args(Provider::Codex).is_err());
        // A program with no launch settings takes none of these.
        assert!(extra(&["--x"]).validate(Provider::None).is_err());
    }

    #[test]
    fn invalid_or_cross_provider_settings_are_rejected() {
        assert!(Options { permission: "yolo".into(), ..Options::default() }.args(Provider::Claude).is_err());
        assert!(Options { chrome: Some(true), ..Options::default() }.args(Provider::Codex).is_err());
        assert!(Options { model: "bad\0value".into(), ..Options::default() }.args(Provider::Claude).is_err());
        assert!(Options { approval: "never".into(), permission: "yolo".into(), ..Options::default() }.args(Provider::Codex).is_err());
    }
}
