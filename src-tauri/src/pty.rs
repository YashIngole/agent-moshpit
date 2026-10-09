//! The terminals: each program runs in a pseudo-terminal of its own, as a child
//! of this app, with no window.
//!
//! This is the same thing a terminal application does. Windows calls it a
//! pseudo console, macOS and Linux a pty; `portable-pty` is one interface over
//! both, so nothing below is written twice.
//!
//! A terminal outlives the window that shows it. What the program printed is
//! kept (the last half megabyte), so a window opened later can be handed the
//! screen as it stands and then follow along.

use crate::model::{now_ms, Millis};
use crate::proctree::Tree;
use crate::status::{Pulse, Signal};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Somewhere a terminal's bytes are sent. Returns false once it has gone away.
pub type Sink = Box<dyn Fn(&[u8]) -> bool + Send>;

/// How much of what a program printed is kept for a window opened later.
const KEPT_BYTES: usize = 512 * 1024;

pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

/// What a terminal has shown, and who is watching it.
#[derive(Default)]
struct Screen {
    kept: VecDeque<u8>,
    /// Each watcher, with the number it was given so it can be taken away again.
    sinks: Vec<(u64, Sink)>,
    pulse: Pulse,
}

impl Screen {
    /// Take in what the program printed. Returns what has to be said back to it
    /// on the terminal's behalf, which is nothing while a window is showing it.
    fn take(&mut self, bytes: &[u8], now: Millis) -> Vec<u8> {
        self.pulse.output(bytes, now);
        self.kept.extend(bytes);
        if self.kept.len() > KEPT_BYTES {
            let extra = self.kept.len() - KEPT_BYTES;
            self.kept.drain(..extra);
            // Start what is kept at the start of a line where one is near, not mid-word.
            if let Some(at) = self.kept.iter().take(4096).position(|&b| b == b'\n') {
                self.kept.drain(..=at);
            }
        }
        self.sinks.retain(|(_, send)| send(bytes));
        let replies = self.pulse.take_replies();
        if self.sinks.is_empty() {
            replies
        } else {
            Vec::new()
        }
    }
}

type Writer = Arc<Mutex<Box<dyn Write + Send>>>;

/// The running half of a terminal. Gone once its program has ended.
struct Live {
    master: Box<dyn MasterPty + Send>,
    writer: Writer,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: u32,
}

struct Term {
    live: Option<Live>,
    screen: Arc<Mutex<Screen>>,
    /// Counts starts, so the end of an earlier program is not taken for the end of this one.
    run: u64,
    /// The size its program last drew at, columns then rows. None for a screen kept
    /// from before, which is already drawn.
    size: Option<(u16, u16)>,
}

/// Called when a program ends: the desk's id, and whether it ended well.
pub type OnExit = Arc<dyn Fn(&str, bool) + Send + Sync>;

pub struct Terminals {
    terms: Mutex<HashMap<String, Term>>,
    tree: Option<Tree>,
    on_exit: OnExit,
    runs: Mutex<u64>,
}

impl Terminals {
    pub fn new(on_exit: OnExit) -> Arc<Terminals> {
        Arc::new(Terminals { terms: Mutex::new(HashMap::new()), tree: Tree::new(), on_exit, runs: Mutex::new(0) })
    }

    /// Start a program in a new terminal under this id. Whoever was watching the
    /// terminal that had this id before keeps watching, and sees the screen wiped.
    pub fn start(self: &Arc<Self>, id: &str, launch: Launch) -> Result<u32, String> {
        let size = PtySize { rows: launch.rows.max(2), cols: launch.cols.max(8), pixel_width: 0, pixel_height: 0 };
        let pair = native_pty_system().openpty(size).map_err(|e| format!("No terminal could be made for it: {e}"))?;

        let mut command = CommandBuilder::new(&launch.program);
        command.args(&launch.args);
        if !launch.cwd.is_empty() {
            command.cwd(&launch.cwd);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        for name in borrowed_names() {
            command.env_remove(name);
        }
        let mut child = pair.slave.spawn_command(command).map_err(|e| format!("It could not be started: {e}"))?;
        drop(pair.slave);

        let pid = child.process_id().unwrap_or(0);
        if let (Some(tree), true) = (&self.tree, pid != 0) {
            tree.add(pid);
        }
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer: Writer = Arc::new(Mutex::new(pair.master.take_writer().map_err(|e| e.to_string())?));
        let killer = child.clone_killer();

        let run = {
            let mut runs = self.runs.lock().unwrap();
            *runs += 1;
            *runs
        };
        let screen = Arc::new(Mutex::new(Screen::default()));
        {
            let mut terms = self.terms.lock().unwrap();
            if let Some(mut old) = terms.remove(id) {
                if let Some(mut live) = old.live.take() {
                    let _ = live.killer.kill();
                }
                // The window's panes follow the desk, not the process.
                let mut before = old.screen.lock().unwrap();
                let mut now = screen.lock().unwrap();
                now.sinks = std::mem::take(&mut before.sinks);
                // Full reset: what the last program left on screen is not this one's.
                now.sinks.retain(|(_, send)| send(b"\x1bc"));
            }
            let size = Some((size.cols, size.rows));
            terms.insert(id.to_string(), Term { live: Some(Live { master: pair.master, writer: writer.clone(), killer, pid }), screen: screen.clone(), run, size });
        }

        std::thread::spawn(move || {
            let mut chunk = [0u8; 16 * 1024];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let replies = screen.lock().unwrap().take(&chunk[..n], now_ms());
                if !replies.is_empty() {
                    let mut writer = writer.lock().unwrap();
                    let _ = writer.write_all(&replies).and_then(|()| writer.flush());
                }
            }
        });

        let (all, id) = (self.clone(), id.to_string());
        std::thread::spawn(move || {
            let ok = child.wait().map(|status| status.success()).unwrap_or(false);
            // Let the last of what it printed through before the terminal is closed.
            std::thread::sleep(std::time::Duration::from_millis(200));
            let ended = {
                let mut terms = all.terms.lock().unwrap();
                match terms.get_mut(&id) {
                    Some(term) if term.run == run => {
                        term.live = None;
                        true
                    }
                    _ => false,
                }
            };
            if ended {
                (all.on_exit)(&id, ok);
            }
        });
        Ok(pid)
    }

    /// A terminal with nothing running in it, showing what a program printed before
    /// the office last quit. Starting a program there wipes it, as any start does.
    pub fn restore(&self, id: &str, printed: &[u8]) {
        let run = {
            let mut runs = self.runs.lock().unwrap();
            *runs += 1;
            *runs
        };
        let screen = Screen { kept: printed.iter().copied().collect(), ..Screen::default() };
        self.terms.lock().unwrap().entry(id.to_string()).or_insert(Term { live: None, screen: Arc::new(Mutex::new(screen)), run, size: None });
    }

    /// What each terminal shows, as it is drawn, with up to `history` lines above it,
    /// to keep on disk so a desk shows it again after a restart. Drawn here, at the size
    /// its program drew at, rather than kept as the program printed it: printed again
    /// into a pane of another size, a program's redraws land in the wrong places.
    pub fn screens(&self, history: usize) -> Vec<(String, Vec<u8>)> {
        let all: Vec<_> = self.terms.lock().unwrap().iter().map(|(id, t)| (id.clone(), t.screen.clone(), t.size)).collect();
        all.into_iter()
            .map(|(id, screen, size)| {
                let printed: Vec<u8> = screen.lock().unwrap().kept.iter().copied().collect();
                let shown = match size {
                    Some((cols, rows)) => drawn(&printed, cols, rows, history),
                    // A screen kept from before is already drawn.
                    None => printed,
                };
                (id, shown)
            })
            .filter(|(_, shown)| !shown.is_empty())
            .collect()
    }

    /// Hand a watcher the screen as it stands, then everything after. The first
    /// thing it is sent is always what was kept, even when that is nothing: a
    /// window must not answer the questions in it, which were answered long ago.
    ///
    /// Returns the number to `detach` with.
    pub fn attach(&self, id: &str, sink: Sink) -> Result<u64, String> {
        let screen = self.terms.lock().unwrap().get(id).map(|t| t.screen.clone()).ok_or("That desk has no terminal yet.")?;
        let token = {
            let mut runs = self.runs.lock().unwrap();
            *runs += 1;
            *runs
        };
        let mut screen = screen.lock().unwrap();
        let kept: Vec<u8> = screen.kept.iter().copied().collect();
        if sink(&kept) {
            screen.sinks.push((token, sink));
        }
        Ok(token)
    }

    /// A watcher has gone: its pane was put away.
    pub fn detach(&self, id: &str, token: u64) {
        let screen = self.terms.lock().unwrap().get(id).map(|t| t.screen.clone());
        if let Some(screen) = screen {
            screen.lock().unwrap().sinks.retain(|(number, _)| *number != token);
        }
    }

    /// Keys from the user.
    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let (writer, screen) = {
            let terms = self.terms.lock().unwrap();
            let term = terms.get(id).ok_or("That desk has no terminal.")?;
            let live = term.live.as_ref().ok_or("Its program is not running.")?;
            (live.writer.clone(), term.screen.clone())
        };
        // One lock at a time: the thread that reads the program takes these two the other way round.
        // The terminal answering for itself (a pane losing focus, say) is not the user, and
        // must not lower a raised hand.
        if crate::status::is_report(data) {
            screen.lock().unwrap().pulse.reported(now_ms());
        } else {
            screen.lock().unwrap().pulse.input(now_ms());
        }
        let mut writer = writer.lock().unwrap();
        writer.write_all(data).and_then(|()| writer.flush()).map_err(|e| e.to_string())
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) {
        let screen = {
            let mut terms = self.terms.lock().unwrap();
            let Some(term) = terms.get_mut(id) else { return };
            let Some(live) = term.live.as_ref() else { return };
            let size = PtySize { rows: rows.max(2), cols: cols.max(8), pixel_width: 0, pixel_height: 0 };
            // The same size again changes nothing, and the program draws nothing.
            if live.master.get_size().is_ok_and(|now| now.rows == size.rows && now.cols == size.cols) {
                return;
            }
            let _ = live.master.resize(size);
            term.size = Some((size.cols, size.rows));
            term.screen.clone()
        };
        // What it prints next is its screen drawn again at the new size.
        screen.lock().unwrap().pulse.resized(now_ms());
    }

    /// End the program. Its last screen stays to be read.
    pub fn stop(&self, id: &str) {
        let mut terms = self.terms.lock().unwrap();
        if let Some(live) = terms.get_mut(id).and_then(|t| t.live.as_mut()) {
            let _ = live.killer.kill();
        }
    }

    /// End the program and forget the terminal.
    pub fn remove(&self, id: &str) {
        let removed = self.terms.lock().unwrap().remove(id);
        if let Some(mut live) = removed.and_then(|t| t.live) {
            let _ = live.killer.kill();
        }
    }

    pub fn stop_all(&self) {
        let mut terms = self.terms.lock().unwrap();
        for term in terms.values_mut() {
            if let Some(live) = term.live.as_mut() {
                let _ = live.killer.kill();
            }
        }
    }

    pub fn running(&self, id: &str) -> bool {
        self.terms.lock().unwrap().get(id).is_some_and(|t| t.live.is_some())
    }

    /// The process id of each running program.
    pub fn pids(&self) -> Vec<(String, u32)> {
        self.terms.lock().unwrap().iter().filter_map(|(id, t)| t.live.as_ref().map(|l| (id.clone(), l.pid))).collect()
    }

    /// What the terminal's own behaviour says, the title its program set, and when it last printed.
    pub fn signal(&self, id: &str, now: Millis) -> Option<(Signal, String, Millis)> {
        let screen = self.terms.lock().unwrap().get(id).map(|t| t.screen.clone())?;
        let screen = screen.lock().unwrap();
        Some((screen.pulse.signal(now), screen.pulse.title.clone(), screen.pulse.printed_ms()))
    }
}

/// What a terminal of this size shows after these bytes, with up to `history` lines
/// that scrolled off above it: each line as it is drawn, colours and all, one after
/// another. Blank lines at the foot are left out.
fn drawn(printed: &[u8], cols: u16, rows: u16, history: usize) -> Vec<u8> {
    let mut parser = vt100::Parser::new(rows.max(2), cols.max(8), history);
    parser.process(printed);
    let screen = parser.screen_mut();
    // Each line on its own: the widest width keeps any line from being joined to the one before.
    let mut lines: Vec<(Vec<u8>, bool)> = Vec::new();
    screen.set_scrollback(usize::MAX);
    let mut above = screen.scrollback();
    while above > 0 {
        screen.set_scrollback(above);
        let take = above.min(usize::from(rows));
        let text: Vec<String> = screen.rows(0, u16::MAX).take(take).collect();
        for (shown, words) in screen.rows_formatted(0, u16::MAX).take(take).zip(text) {
            lines.push((shown, words.trim().is_empty()));
        }
        above -= take;
    }
    screen.set_scrollback(0);
    let text: Vec<String> = screen.rows(0, u16::MAX).collect();
    for (shown, words) in screen.rows_formatted(0, u16::MAX).zip(text) {
        lines.push((shown, words.trim().is_empty()));
    }
    while lines.last().is_some_and(|(_, blank)| *blank) {
        lines.pop();
    }
    let mut out = Vec::new();
    for (i, (shown, _)) in lines.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\x1b[m");
        out.extend_from_slice(shown);
    }
    if !out.is_empty() {
        out.extend_from_slice(b"\x1b[m\r\n");
    }
    out
}

/// Names in this app's own environment that say "you are running inside an
/// agent's session". They are true of this app when it was started from one (a
/// developer running it from Claude Code, say) and false of the programs it
/// starts, which would otherwise take themselves for a session inside a session.
fn borrowed_names() -> Vec<String> {
    if std::env::var_os("CLAUDECODE").is_none() {
        return Vec::new();
    }
    std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| name == "CLAUDECODE" || name == "CLAUDE_PID" || name.starts_with("CLAUDE_CODE_") || name.starts_with("CLAUDE_AGENT_SDK_"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// A program every system has, that prints a word and ends.
    fn echo(word: &str) -> Launch {
        let (program, args) = if cfg!(windows) {
            ("cmd.exe".to_string(), vec!["/d".to_string(), "/c".to_string(), format!("echo {word}")])
        } else {
            ("/bin/sh".to_string(), vec!["-c".to_string(), format!("echo {word}")])
        };
        Launch { program, args, cwd: String::new(), cols: 80, rows: 24 }
    }

    /// A shell that waits to be typed into.
    fn shell() -> Launch {
        let (program, args) = if cfg!(windows) { ("cmd.exe".to_string(), vec!["/d".to_string(), "/q".to_string()]) } else { ("/bin/sh".to_string(), vec![]) };
        Launch { program, args, cwd: String::new(), cols: 80, rows: 24 }
    }

    fn watcher() -> (Sink, Arc<Mutex<Vec<u8>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        (Box::new(move |bytes: &[u8]| {
            into.lock().unwrap().extend_from_slice(bytes);
            true
        }), seen)
    }

    fn wait_for(seen: &Arc<Mutex<Vec<u8>>>, word: &str) -> bool {
        for _ in 0..100 {
            if String::from_utf8_lossy(&seen.lock().unwrap()).contains(word) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn a_program_runs_in_a_terminal_and_its_end_is_reported() {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let terms = Terminals::new(Arc::new(move |id: &str, ok: bool| {
            let _ = tx.lock().unwrap().send((id.to_string(), ok));
        }));
        let pid = terms.start("a", echo("moshpit-says-hello")).unwrap();
        assert!(pid != 0);
        assert_eq!(rx.recv_timeout(Duration::from_secs(20)).unwrap(), ("a".to_string(), true));
        assert!(!terms.running("a"));
        // A watcher who arrives after the end is still shown what was printed.
        let (sink, seen) = watcher();
        terms.attach("a", sink).unwrap();
        assert!(wait_for(&seen, "moshpit-says-hello"), "{:?}", String::from_utf8_lossy(&seen.lock().unwrap()));
        assert!(terms.write("a", b"x").is_err());
    }

    #[test]
    fn what_is_typed_reaches_the_program() {
        let terms = Terminals::new(Arc::new(|_: &str, _: bool| {}));
        terms.start("a", shell()).unwrap();
        // This watcher cannot answer what a terminal is asked the way a window does,
        // so it arrives once the shell is up and nothing is being asked any more.
        for _ in 0..100 {
            if terms.signal("a", now_ms()).is_some_and(|(signal, _, _)| signal != Signal::Silent) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(400));
        let (sink, seen) = watcher();
        terms.attach("a", sink).unwrap();
        terms.write("a", b"echo typed-and-run\r\n").unwrap();
        assert!(wait_for(&seen, "typed-and-run"), "{:?}", String::from_utf8_lossy(&seen.lock().unwrap()));
        assert!(terms.running("a"));
        assert_eq!(terms.pids().len(), 1);
        terms.resize("a", 100, 30);
        // A watcher who leaves is sent nothing more.
        let (sink, late) = watcher();
        let token = terms.attach("a", sink).unwrap();
        terms.detach("a", token);
        let before = late.lock().unwrap().len();
        terms.write("a", b"echo after-leaving\r\n").unwrap();
        assert!(wait_for(&seen, "after-leaving"));
        assert_eq!(late.lock().unwrap().len(), before);
        terms.remove("a");
        assert!(!terms.running("a"));
        assert!(terms.attach("a", Box::new(|_: &[u8]| true)).is_err());
    }

    #[test]
    fn a_screen_from_before_is_shown_until_a_program_starts() {
        let terms = Terminals::new(Arc::new(|_: &str, _: bool| {}));
        terms.restore("a", b"line one\r\nwhat it said last\r\n");
        assert!(!terms.running("a"));
        let (sink, seen) = watcher();
        terms.attach("a", sink).unwrap();
        assert!(wait_for(&seen, "what it said last"));
        // Kept on disk again as it is: it was drawn already.
        let screens = terms.screens(500);
        assert_eq!(screens, vec![("a".to_string(), b"line one\r\nwhat it said last\r\n".to_vec())]);
        // A program started there wipes it.
        terms.start("a", echo("fresh-start")).unwrap();
        assert!(wait_for(&seen, "\x1bc"));
    }

    #[test]
    fn a_screen_is_kept_as_it_was_drawn_not_as_it_was_printed() {
        // A program that draws its last lines again in place, as Claude Code and Codex do.
        let printed = b"history 1\r\nhistory 2\r\n> draft one\r\n  thinking\x1b[1A\r\x1b[J\x1b[1m> final answer\x1b[0m\r\n  done\r\n";
        let shown = drawn(printed, 40, 5, 100);
        let text = String::from_utf8_lossy(&shown);
        assert!(!text.contains("draft one") && !text.contains("thinking"), "{text:?}");
        // Read back by a terminal of any size, it is those lines, in order, and the bold stays.
        let mut again = vt100::Parser::new(10, 60, 0);
        again.process(&shown);
        let lines: Vec<String> = again.screen().rows(0, 60).map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect();
        assert_eq!(lines, ["history 1", "history 2", "> final answer", "  done"]);
        assert!(again.screen().cell(2, 2).unwrap().bold());

        // Lines that scrolled off the top are kept above the screen.
        let long: Vec<u8> = (1..=12).flat_map(|n| format!("line {n}\r\n").into_bytes()).collect();
        let mut back = vt100::Parser::new(20, 40, 0);
        back.process(&drawn(&long, 40, 5, 100));
        let lines: Vec<String> = back.screen().rows(0, 40).map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect();
        assert_eq!(lines, (1..=12).map(|n| format!("line {n}")).collect::<Vec<_>>());
    }

    #[test]
    fn a_program_that_is_not_there_says_so() {
        let terms = Terminals::new(Arc::new(|_: &str, _: bool| {}));
        let missing = Launch { program: "no-such-program-moshpit".into(), args: vec![], cwd: String::new(), cols: 80, rows: 24 };
        assert!(terms.start("a", missing).is_err());
        assert!(!terms.running("a"));
    }
}
