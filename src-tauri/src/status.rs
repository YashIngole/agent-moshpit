//! Telling whether a program is working, idle or waiting, without reading its screen.
//!
//! Two ways, and the second works for any program at all:
//!
//! - Claude Code keeps a small file about each of its running sessions and says
//!   in it whether it is busy, idle or waiting. That is read when it is there.
//! - Every terminal program shows what it is doing by how it behaves: one that is
//!   working keeps printing (a spinner, a stream of words), one that is waiting
//!   goes quiet, and one that wants the user rings the bell or sends a notice.
//!
//! Startup trust dialogs are the one narrow screen-text exception: configuration
//! alone cannot tell whether a CLI actually stopped to ask a question.

use crate::harness::TrustFrom;
use crate::model::Millis;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// What one look at a running program found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// Nothing has been printed yet.
    Silent,
    Working,
    Quiet,
    /// It wants the user. The text says what for, when it said.
    Asked(String),
    /// It announced that it has finished.
    Finished,
}

/// A program printing this recently is at work.
const BUSY_FOR_MS: Millis = 2_500;
/// Printing this soon after a key was pressed is the key being shown, not work.
const ECHO_MS: Millis = 450;
/// After the terminal changes size, the screen is drawn again: printing is the redraw until
/// it pauses this long. Some programs (Codex) draw in bursts, or a moment after the resize.
const REDRAW_PAUSE_MS: Millis = 300;
/// The longest a redraw is taken to last, so work that starts at once is not hidden for long.
const REDRAW_MOST_MS: Millis = 3_000;

/// A redraw after a resize: when the resize was, and when it last printed.
#[derive(Debug, Clone, Copy)]
struct Redraw {
    since: Millis,
    last: Option<Millis>,
}

/// What a terminal has been doing, kept up to date as its bytes go by.
#[derive(Debug, Default)]
pub struct Pulse {
    first_output_ms: Millis,
    last_output_ms: Millis,
    last_input_ms: Millis,
    /// When the terminal last told the program something of its own: focus, the cursor's place.
    last_report_ms: Millis,
    /// The redraw after a resize, while it lasts.
    redraw: Option<Redraw>,
    /// The latest bell or notice, until the user types again.
    notice: Option<String>,
    /// What the program last set as its window title.
    pub title: String,
    /// The start of an escape sequence that has not ended yet.
    pending: Vec<u8>,
    state: Parse,
    /// Answers to what the program asked its terminal, for when no window is there to give them.
    replies: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Parse {
    #[default]
    Text,
    Escape,
    /// Inside `ESC [`, which runs to a letter.
    Control,
    /// Inside `ESC ]`, which runs to a bell or to `ESC \`.
    Command,
    CommandEscape,
}

impl Pulse {
    /// The program printed something.
    pub fn output(&mut self, bytes: &[u8], now: Millis) {
        if bytes.is_empty() {
            return;
        }
        if self.first_output_ms == 0 {
            self.first_output_ms = now;
        }
        // What comes straight back after a key is that key being drawn, and what
        // comes after a resize, until it pauses, is the same screen drawn at its new size.
        let redrawing = match &mut self.redraw {
            Some(redraw) if now.saturating_sub(redraw.since) < REDRAW_MOST_MS && redraw.last.is_none_or(|last| now.saturating_sub(last) < REDRAW_PAUSE_MS) => {
                redraw.last = Some(now);
                true
            }
            _ => {
                self.redraw = None;
                false
            }
        };
        // And what comes straight after the terminal reported something (a pane losing the
        // keyboard, say) is the program answering its terminal: Codex draws an empty frame.
        let answering = now.saturating_sub(self.last_input_ms) <= ECHO_MS || now.saturating_sub(self.last_report_ms) <= ECHO_MS;
        if !answering && !redrawing {
            self.last_output_ms = now;
        }
        for &byte in bytes {
            self.step(byte);
        }
    }

    /// What a terminal would answer to the questions the program has put to it
    /// since this was last called: where the cursor is, and what kind of terminal
    /// this is. A window that is showing the terminal answers them itself; these
    /// are for when there is none. Windows asks the first as soon as a program
    /// starts and shows nothing until it is answered.
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    /// The user typed something. Whatever was asked for is being answered.
    pub fn input(&mut self, now: Millis) {
        self.last_input_ms = now;
        self.notice = None;
    }

    /// The terminal told the program something of its own accord: focus coming or going,
    /// where the cursor is. Nobody typed, so nothing that was asked has been answered.
    pub fn reported(&mut self, now: Millis) {
        self.last_report_ms = now;
    }

    /// The terminal changed size. The program draws its screen again, which is not work;
    /// unless it was at work already, when what it prints next is more of that.
    pub fn resized(&mut self, now: Millis) {
        let busy = self.last_output_ms > 0 && now.saturating_sub(self.last_output_ms) < BUSY_FOR_MS;
        if !busy {
            self.redraw = Some(Redraw { since: now, last: None });
        }
    }

    /// When it last printed something that was not a key drawn back or a redraw. Zero until then.
    pub fn printed_ms(&self) -> Millis {
        self.last_output_ms
    }

    pub fn signal(&self, now: Millis) -> Signal {
        if self.first_output_ms == 0 {
            return Signal::Silent;
        }
        if let Some(text) = &self.notice {
            return if wants_an_answer(text) { Signal::Asked(text.clone()) } else { Signal::Finished };
        }
        if self.last_output_ms > 0 && now.saturating_sub(self.last_output_ms) < BUSY_FOR_MS {
            Signal::Working
        } else {
            Signal::Quiet
        }
    }

    fn step(&mut self, byte: u8) {
        match self.state {
            Parse::Text => match byte {
                0x1b => self.state = Parse::Escape,
                0x07 => self.notice = Some(String::new()),
                _ => {}
            },
            Parse::Escape => {
                self.pending.clear();
                self.state = match byte {
                    b']' => Parse::Command,
                    b'[' => Parse::Control,
                    _ => Parse::Text,
                };
            }
            Parse::Control => match byte {
                // Numbers and marks before the letter that ends it.
                0x20..=0x3f if self.pending.len() < 32 => self.pending.push(byte),
                0x40..=0x7e => {
                    match (byte, self.pending.as_slice()) {
                        (b'n', b"6") => self.replies.extend_from_slice(b"\x1b[1;1R"),
                        (b'c', b"" | b"0") => self.replies.extend_from_slice(b"\x1b[?1;2c"),
                        _ => {}
                    }
                    self.pending.clear();
                    self.state = Parse::Text;
                }
                _ => {
                    self.pending.clear();
                    self.state = if byte == 0x1b { Parse::Escape } else { Parse::Text };
                }
            },
            Parse::Command => match byte {
                0x07 => self.end_command(),
                0x1b => self.state = Parse::CommandEscape,
                // A command that never ends is not one; give up on it rather than keep it all.
                _ if self.pending.len() > 2048 => {
                    self.pending.clear();
                    self.state = Parse::Text;
                }
                _ => self.pending.push(byte),
            },
            Parse::CommandEscape => {
                if byte == b'\\' {
                    self.end_command();
                } else {
                    self.pending.clear();
                    self.state = Parse::Text;
                }
            }
        }
    }

    fn end_command(&mut self) {
        let text = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        self.state = Parse::Text;
        let (code, rest) = text.split_once(';').unwrap_or((&text, ""));
        match code {
            "0" | "2" => self.title = rest.trim().to_string(),
            // 9 is a desktop notice, except "9;4;…", which is a progress bar.
            "9" if !rest.starts_with("4;") => self.notice = Some(rest.trim().to_string()),
            "777" => {
                // 777;notify;title;body
                let mut parts = rest.splitn(3, ';');
                if parts.next() == Some("notify") {
                    let title = parts.next().unwrap_or("");
                    let body = parts.next().unwrap_or("");
                    self.notice = Some(if body.is_empty() { title.trim().to_string() } else { body.trim().to_string() });
                }
            }
            _ => {}
        }
    }
}

/// Whether what the window sends is only the terminal speaking for itself: focus
/// coming and going, where the cursor is, what kind of terminal it is, which
/// colours it shows. None of that is the user typing, so none of it answers a
/// question. (Windows turns focus reports on for every program, so a pane that
/// loses focus sends one.)
pub fn is_report(data: &[u8]) -> bool {
    let mut rest = data;
    while !rest.is_empty() {
        match report_len(rest) {
            Some(n) => rest = &rest[n..],
            None => return false,
        }
    }
    true
}

/// How long the report at the start of `data` is, if it starts with one.
fn report_len(data: &[u8]) -> Option<usize> {
    match data {
        // Focus in, focus out.
        [0x1b, b'[', b'I' | b'O', ..] => Some(3),
        [0x1b, b'[', rest @ ..] => {
            let end = rest.iter().position(|b| (0x40..=0x7e).contains(b))?;
            let (params, last) = (&rest[..end], rest[end]);
            if !params.iter().all(|b| (0x20..=0x3f).contains(b)) {
                return None;
            }
            let report = match last {
                // Where the cursor is.
                b'R' => params.contains(&b';'),
                // What kind of terminal this is.
                b'c' => matches!(params.first(), Some(b'?' | b'>' | b'=')),
                // "All is well", and a mode as it stands.
                b'n' => params == b"0",
                b'y' => params.ends_with(b"$"),
                // The size of the window.
                b't' => !params.is_empty(),
                _ => false,
            };
            report.then_some(end + 3)
        }
        // Colours and settings, answered as a command: to a bell, or to ESC \.
        [0x1b, b']' | b'P', rest @ ..] => {
            let bell = rest.iter().position(|&b| b == 0x07).map(|at| at + 3);
            let st = rest.windows(2).position(|w| w == b"\x1b\\").map(|at| at + 4);
            match (bell, st) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            }
        }
        _ => None,
    }
}

/// A notice with no words (a bare bell) is taken as a question: it is the louder
/// reading, and a raised hand that was not needed costs one glance.
fn wants_an_answer(notice: &str) -> bool {
    let text = notice.to_lowercase();
    if text.is_empty() {
        return true;
    }
    let done = ["complete", "finished", "done", "ready"].iter().any(|w| text.contains(w));
    let asks = ["approv", "permission", "confirm", "waiting", "input", "question", "needs", "allow", "review"].iter().any(|w| text.contains(w));
    asks || !done
}

// ── Claude Code's own account of its sessions ──────────────────────────────

/// What Claude Code writes to `~/.claude/sessions/<pid>.json` while it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClaudeSession {
    pub session_id: String,
    pub cwd: String,
    pub name: String,
    /// `busy`, `idle` or `waiting`, as of Claude Code 2.1.290.
    pub status: String,
    /// What it is waiting for: "dialog open", "input needed", and a few more.
    pub waiting_for: String,
}

pub fn claude_session_file(home: &Path, pid: u32) -> PathBuf {
    config_dir(home, "CLAUDE_CONFIG_DIR", ".claude").join("sessions").join(format!("{pid}.json"))
}

fn config_dir(home: &Path, variable: &str, fallback: &str) -> PathBuf {
    configured_dir(home, std::env::var_os(variable).as_deref(), fallback)
}

fn configured_dir(home: &Path, chosen: Option<&std::ffi::OsStr>, fallback: &str) -> PathBuf {
    chosen.filter(|p| !p.is_empty()).map_or_else(|| home.join(fallback), PathBuf::from)
}

pub fn read_claude_session(home: &Path, pid: u32) -> Option<ClaudeSession> {
    serde_json::from_str(&std::fs::read_to_string(claude_session_file(home, pid)).ok()?).ok()
}

/// `None` when the file says nothing this version understands: the terminal's
/// own behaviour is used instead.
pub fn claude_signal(session: &ClaudeSession) -> Option<Signal> {
    if !session.waiting_for.is_empty() || session.status == "waiting" {
        let what = match session.waiting_for.as_str() {
            "dialog open" | "" => "Waiting for your answer in the terminal".to_string(),
            "input needed" => "Asked you a question".to_string(),
            other => format!("Waiting: {other}"),
        };
        return Some(Signal::Asked(what));
    }
    match session.status.as_str() {
        "busy" => Some(Signal::Working),
        // This is an explicit ready state, unlike a pause in terminal output.
        "idle" => Some(Signal::Finished),
        _ => None,
    }
}

// ── which conversation Codex began ─────────────────────────────────────────

/// Codex is asked to put its own session id in its terminal title. Recent CLIs
/// shorten it; resolve that prefix against rollout metadata, never against the
/// newest session in a folder. Ambiguous or unsupported titles remain unknown.
pub fn codex_session(home: &Path, title: &str) -> Option<String> {
    session_in(&config_dir(home, "CODEX_HOME", ".codex").join("sessions"), title)
}

pub fn session_prefix(title: &str) -> Option<&str> {
    let title = title.trim();
    let prefix = title.strip_suffix("...").or_else(|| title.strip_suffix('…')).unwrap_or(title);
    (prefix.len() >= 24 && prefix.len() <= 36 && prefix.bytes().enumerate().all(|(i, b)| if matches!(i, 8 | 13 | 18 | 23) { b == b'-' } else { b.is_ascii_hexdigit() })).then_some(prefix)
}

pub fn session_title_matches(title: &str, id: &str) -> bool {
    session_prefix(title).is_some_and(|prefix| id.starts_with(prefix))
}

fn session_in(root: &Path, title: &str) -> Option<String> {
    let prefix = session_prefix(title)?;
    let mut dirs = vec![root.to_path_buf()];
    let mut matches = std::collections::HashSet::new();
    // The root and YYYY/MM/DD; do not follow directory symlinks or scan unrelated files.
    for depth in 0..=3 {
        let mut next = Vec::new();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() && depth < 3 {
                    next.push(entry.path());
                } else if kind.is_file() && entry.file_name().to_string_lossy().contains(prefix) && entry.path().extension().is_some_and(|e| e == "jsonl") {
                    if let Some(id) = first_line(&entry.path()).and_then(|line| codex_meta(&line)) {
                        if id.len() == 36 && session_prefix(&id).is_some() && id.starts_with(prefix) {
                            matches.insert(id);
                        }
                    }
                }
            }
        }
        dirs = next;
    }
    (matches.len() == 1).then(|| matches.into_iter().next()).flatten()
}

fn first_line(path: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader, Read};
    let mut line = String::new();
    BufReader::new(std::fs::File::open(path).ok()?.take(256 * 1024)).read_line(&mut line).ok()?;
    Some(line)
}

/// The session id on a `session_meta` line.
fn codex_meta(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    let payload = value.get("payload")?;
    let id = payload.get("session_id").or_else(|| payload.get("id"))?.as_str()?;
    (!id.is_empty()).then(|| id.to_string())
}

// ── whether it will first ask to trust the folder ──────────────────────────

/// A trust question actually visible in the startup screen. Unknown wording is
/// left to the CLI's signals, never guessed from a configuration file.
pub fn trust_prompt(from: TrustFrom, screen: &str) -> bool {
    let words = screen.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    match from {
        TrustFrom::Claude => words.contains("yes, i trust this folder") && words.contains("no, exit"),
        TrustFrom::Codex => words.contains("do you trust") && (words.contains("yes, continue") || words.contains("yes, i trust")) && (words.contains("no, quit") || words.contains("no, exit")),
        TrustFrom::None => false,
    }
}

/// Folders are compared the way Windows does: no matter the case, the slashes
/// or a `\\?\` in front.
fn project_key(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let key = path.to_string_lossy().trim_start_matches("\\\\?\\").replace('\\', "/");
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

// ── which project and branch a folder is ───────────────────────────────────

/// The name of the project a folder belongs to, and the branch checked out there.
/// A git worktree answers with the name of the project it is a copy of.
pub fn project(cwd: &Path) -> (String, String, String) {
    let fallback = || (project_key(cwd), base_name(&cwd.to_string_lossy()), String::new());
    let Some((root, git)) = cwd.ancestors().find_map(|dir| {
        let git = dir.join(".git");
        git.exists().then(|| (dir.to_path_buf(), git))
    }) else {
        return fallback();
    };
    if git.is_dir() {
        return (project_key(&root), base_name(&root.to_string_lossy()), branch_in(&git.join("HEAD")));
    }
    // A worktree: `.git` is a file that says "gitdir: <main>/.git/worktrees/<name>".
    let Some(pointer) = std::fs::read_to_string(&git).ok().and_then(|t| t.trim().strip_prefix("gitdir:").map(|p| PathBuf::from(p.trim()))) else {
        return fallback();
    };
    let pointer = if pointer.is_absolute() { pointer } else { root.join(pointer) };
    let common = std::fs::read_to_string(pointer.join("commondir")).ok().map(|p| pointer.join(p.trim())).and_then(|p| p.canonicalize().ok());
    let main = common.as_deref().and_then(Path::parent).or_else(|| pointer.ancestors().find(|p| p.file_name().is_some_and(|n| n == ".git")).and_then(Path::parent));
    let name = main.map(|m| base_name(&m.to_string_lossy())).unwrap_or_else(|| base_name(&root.to_string_lossy()));
    (project_key(main.unwrap_or(&root)), name, branch_in(&pointer.join("HEAD")))
}

fn branch_in(head: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(head) else {
        return String::new();
    };
    let text = text.trim();
    match text.strip_prefix("ref: refs/heads/") {
        Some(branch) => branch.to_string(),
        // Not on a branch: the start of the commit it stands on.
        None => text.chars().take(7).collect(),
    }
}

pub fn base_name(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printing_is_work_and_silence_is_not() {
        let mut pulse = Pulse::default();
        assert_eq!(pulse.signal(1_000), Signal::Silent);
        pulse.output(b"Thinking", 1_000);
        assert_eq!(pulse.signal(1_200), Signal::Working);
        assert_eq!(pulse.signal(1_000 + BUSY_FOR_MS + 1), Signal::Quiet);
    }

    #[test]
    fn a_key_being_shown_is_not_work() {
        let mut pulse = Pulse::default();
        pulse.output(b"> ", 1_000);
        // Typing at ten keys a second, each one drawn straight back.
        for i in 0..40u64 {
            let at = 10_000 + i * 100;
            pulse.input(at);
            pulse.output(b"x", at + 20);
        }
        assert_eq!(pulse.signal(14_100), Signal::Quiet);
        // Enter, and then the program keeps printing on its own.
        pulse.input(14_200);
        pulse.output(b"\r\n", 14_210);
        pulse.output(b"Working...", 15_000);
        assert_eq!(pulse.signal(15_100), Signal::Working);
    }

    #[test]
    fn a_bell_is_a_raised_hand_until_the_user_types() {
        let mut pulse = Pulse::default();
        pulse.output(b"Allow this command?\x07", 1_000);
        assert_eq!(pulse.signal(60_000), Signal::Asked(String::new()));
        pulse.input(61_000);
        assert_eq!(pulse.signal(70_000), Signal::Quiet);
    }

    #[test]
    fn the_terminal_speaking_for_itself_is_not_the_user() {
        // Focus coming and going, the cursor's place, the kind of terminal, a colour, a mode.
        for report in [
            &b"\x1b[O"[..],
            b"\x1b[I",
            b"\x1b[O\x1b[I",
            b"\x1b[12;40R",
            b"\x1b[?1;2c",
            b"\x1b[>0;276;0c",
            b"\x1b[0n",
            b"\x1b[?2004;1$y",
            b"\x1b[8;24;80t",
            b"\x1b]11;rgb:0b0b/0d0d/1010\x07",
            b"\x1b]10;rgb:d5d5/dada/e2e2\x1b\\",
            b"\x1bP1$r0m\x1b\\",
            b"",
        ] {
            assert!(is_report(report), "{:?}", String::from_utf8_lossy(report));
        }
        // Keys, a paste, a click in a program that follows the mouse.
        for typed in [&b"y"[..], b"\r", b"\x1b", b"\x1b[A", b"\x1b[1;5D", b"\x1b[15~", b"\x1b]", b"\x1bP", b"\x1b[200~text\x1b[201~", b"\x1b[<0;10;5M", b"\x1b[Ox"] {
            assert!(!is_report(typed), "{:?}", String::from_utf8_lossy(typed));
        }
    }

    #[test]
    fn a_screen_drawn_again_after_a_resize_is_not_work() {
        let mut pulse = Pulse::default();
        pulse.output(b"> ", 1_000);
        assert_eq!(pulse.signal(10_000), Signal::Quiet);
        pulse.resized(10_000);
        pulse.output(b"\x1b[2J the whole screen again", 10_100);
        assert_eq!(pulse.signal(10_200), Signal::Quiet);
        // Printing well after the resize is work again.
        pulse.output(b"Thinking", 12_000);
        assert_eq!(pulse.signal(12_100), Signal::Working);
    }

    #[test]
    fn answering_the_terminal_is_not_work() {
        // As traced from Codex 0.161: a pane beside it takes the keyboard, the window reports
        // focus out, Codex draws an empty frame at once, and then the resize arrives.
        let mut pulse = Pulse::default();
        pulse.output(b"> ", 1_000);
        pulse.reported(50_024);
        pulse.output(b"[?2026h", 50_040);
        pulse.output(b"[?2026l", 50_040);
        assert_eq!(pulse.signal(50_100), Signal::Quiet);
        // So it is not taken to be at work when the resize comes, and its redraw is a redraw.
        pulse.resized(50_613);
        pulse.output(b"[?25l[H[K the whole screen", 50_614);
        pulse.output(b"[?2026h[0 q[?2026l", 50_699);
        assert_eq!(pulse.signal(51_000), Signal::Quiet);
        // A report does not hide work that goes on after it.
        pulse.reported(60_000);
        pulse.output(b"Working", 60_010);
        pulse.output(b"Working.", 60_600);
        assert_eq!(pulse.signal(60_700), Signal::Working);
        // And it is not typing: a raised hand stays up.
        let mut asked = Pulse::default();
        asked.output(b"Allow?", 1_000);
        asked.reported(2_000);
        assert_eq!(asked.signal(3_000), Signal::Asked(String::new()));
    }

    #[test]
    fn a_redraw_lasts_until_it_pauses_however_long_or_late_it_is() {
        let mut pulse = Pulse::default();
        pulse.output(b"> ", 1_000);
        // Drawn in bursts a quarter of a second apart, for longer than a second and a half.
        pulse.resized(10_000);
        for at in (10_100..=11_600).step_by(250) {
            pulse.output(b"\x1b[H a line of the screen", at);
            assert_eq!(pulse.signal(at + 50), Signal::Quiet, "at {at}");
        }
        // After a pause, printing is work again.
        pulse.output(b"Thinking", 12_200);
        assert_eq!(pulse.signal(12_300), Signal::Working);

        // Drawn a while after the resize: still the redraw.
        let mut late = Pulse::default();
        late.output(b"> ", 1_000);
        late.resized(20_000);
        late.output(b"\x1b[2J the whole screen", 21_500);
        assert_eq!(late.signal(21_600), Signal::Quiet);

        // Resized in the middle of work: what follows is more of the work.
        let mut busy = Pulse::default();
        busy.output(b"Working...", 30_000);
        busy.resized(30_500);
        busy.output(b"Still working...", 31_000);
        assert_eq!(busy.signal(33_000), Signal::Working);

        // A redraw that never pauses is not taken for one for ever.
        let mut endless = Pulse::default();
        endless.output(b"> ", 1_000);
        endless.resized(40_000);
        for at in (40_100..=44_000).step_by(100) {
            endless.output(b".", at);
        }
        assert_eq!(endless.signal(44_050), Signal::Working);
    }

    #[test]
    fn notices_are_read_even_in_pieces() {
        let mut pulse = Pulse::default();
        pulse.output(b"\x1b]0;codex: api\x07text\x1b]9;Appro", 1_000);
        pulse.output(b"val requested: rm -rf dist\x1b\\", 1_010);
        assert_eq!(pulse.title, "codex: api");
        assert_eq!(pulse.signal(50_000), Signal::Asked("Approval requested: rm -rf dist".into()));

        let mut done = Pulse::default();
        done.output(b"\x1b]9;Agent turn complete\x07", 1_000);
        assert_eq!(done.signal(50_000), Signal::Finished);

        let mut other = Pulse::default();
        other.output(b"\x1b]777;notify;Gemini;Waiting for your input\x07", 1_000);
        assert_eq!(other.signal(50_000), Signal::Asked("Waiting for your input".into()));

        // A progress bar and a colour query are neither.
        let mut quiet = Pulse::default();
        quiet.output(b"\x1b]9;4;1;50\x07\x1b]11;?\x1b\\", 1_000);
        assert_eq!(quiet.signal(50_000), Signal::Quiet);
    }

    #[test]
    fn questions_put_to_the_terminal_get_an_answer() {
        let mut pulse = Pulse::default();
        // Where is the cursor? Asked in two pieces, as it can arrive.
        pulse.output(b"\x1b[6", 1_000);
        assert!(pulse.take_replies().is_empty());
        pulse.output(b"n", 1_001);
        assert_eq!(pulse.take_replies(), b"\x1b[1;1R");
        // Answered once, not again.
        assert!(pulse.take_replies().is_empty());
        // What are you? Both spellings.
        pulse.output(b"\x1b[c\x1b[0c", 1_002);
        assert_eq!(pulse.take_replies(), b"\x1b[?1;2c\x1b[?1;2c");
        // Colours, cursor moves and other questions are not these.
        pulse.output(b"\x1b[31m\x1b[2J\x1b[10;4H\x1b[?6n\x1b[5n\x1b[>c", 1_003);
        assert!(pulse.take_replies().is_empty());
    }

    #[test]
    fn claude_says_what_it_is_doing() {
        let session = |status: &str, waiting: &str| ClaudeSession { status: status.into(), waiting_for: waiting.into(), ..Default::default() };
        assert_eq!(claude_signal(&session("busy", "")), Some(Signal::Working));
        assert_eq!(claude_signal(&session("idle", "")), Some(Signal::Finished));
        assert_eq!(claude_signal(&session("waiting", "dialog open")), Some(Signal::Asked("Waiting for your answer in the terminal".into())));
        assert_eq!(claude_signal(&session("busy", "sandbox request")), Some(Signal::Asked("Waiting: sandbox request".into())));
        // A word a later version invents is not guessed at.
        assert_eq!(claude_signal(&session("pondering", "")), None);
        let read: ClaudeSession =
            serde_json::from_str(r#"{"pid":1992,"sessionId":"d647","cwd":"C:\\p","name":"Roadmap","status":"busy","kind":"interactive","updatedAt":1}"#).unwrap();
        assert_eq!((read.session_id.as_str(), read.name.as_str(), read.status.as_str()), ("d647", "Roadmap", "busy"));
    }

    #[test]
    fn session_identity_comes_from_each_terminal_not_shared_cwd() {
        let dir = std::env::temp_dir().join(format!("moshpit-identity-{}", std::process::id()));
        let day = dir.join("2026/10/09");
        std::fs::create_dir_all(&day).unwrap();
        let ids = ["01a1208b-04bd-7501-b17d-5d25a0000001", "01a1208b-04be-7501-b17d-5d25a0000002"];
        for id in ids {
            std::fs::write(day.join(format!("rollout-{id}.jsonl")), serde_json::json!({"type":"session_meta","payload":{"id":id,"cwd":"same-folder"}}).to_string()).unwrap();
        }
        for id in ids {
            assert_eq!(session_in(&dir, &format!("{}...", &id[..27])), Some(id.into()));
        }
        assert_eq!(session_in(&dir, "codex: same-folder"), None);
        assert_eq!(session_in(&dir, "01a1208b..."), None);
        let collision = "01a1208b-04bd-7501-b17d-5d25a0000003";
        std::fs::write(day.join(format!("rollout-{collision}.jsonl")), serde_json::json!({"type":"session_meta","payload":{"id":collision}}).to_string()).unwrap();
        assert_eq!(session_in(&dir, &format!("{}...", &ids[0][..27])), None);
        assert_eq!(session_in(&dir, ids[0]), Some(ids[0].into()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cli_config_roots_honor_custom_homes() {
        let home = Path::new("home");
        assert_eq!(configured_dir(home, None, ".codex"), home.join(".codex"));
        assert_eq!(configured_dir(home, Some(std::ffi::OsStr::new("custom")), ".codex"), Path::new("custom"));
        assert_eq!(configured_dir(home, Some(std::ffi::OsStr::new("")), ".claude"), home.join(".claude"));
    }

    #[test]
    fn only_a_visible_trust_dialog_raises_a_hand() {
        assert!(trust_prompt(TrustFrom::Claude, "Accessing workspace: ... Yes, I trust this folder\nNo, exit"));
        assert!(trust_prompt(TrustFrom::Codex, "Do you trust the contents of this directory?\n1. Yes, continue\n2. No, quit"));
        assert!(!trust_prompt(TrustFrom::Codex, "OpenAI Codex /permissions choose what Codex is allowed to do Ask Codex to do anything"));
        assert!(!trust_prompt(TrustFrom::Claude, "Yes, I trust this folder"));
        assert!(!trust_prompt(TrustFrom::None, "Do you trust this directory? Yes, continue No, quit"));
    }

    #[test]
    fn a_folder_knows_its_project_and_branch() {
        let dir = std::env::temp_dir().join(format!("moshpit-project-{}", std::process::id()));
        let main = dir.join("shop");
        let copy = dir.join("copies").join("fix-total");
        std::fs::create_dir_all(main.join(".git").join("worktrees").join("fix-total")).unwrap();
        std::fs::create_dir_all(main.join("src").join("cart")).unwrap();
        std::fs::create_dir_all(&copy).unwrap();
        std::fs::write(main.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(main.join(".git").join("worktrees").join("fix-total").join("HEAD"), "ref: refs/heads/fix/checkout-total\n").unwrap();
        std::fs::write(copy.join(".git"), format!("gitdir: {}\n", main.join(".git").join("worktrees").join("fix-total").display())).unwrap();

        assert_eq!(project(&main.join("src").join("cart")), (project_key(&main), "shop".to_string(), "main".to_string()));
        assert_eq!(project(&copy), (project_key(&main), "shop".to_string(), "fix/checkout-total".to_string()));
        assert_eq!(project(&dir.join("copies")), (project_key(&dir.join("copies")), "copies".to_string(), String::new()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
