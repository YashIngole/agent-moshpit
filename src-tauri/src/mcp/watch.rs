//! Connections a session's program holds open to the office, so that the office can
//! speak first. A call is asked and answered and gone; these stay:
//!
//! - a **watch**, from the session's MCP subprocess for as long as it runs. While it is
//!   open the subprocess is known to be alive, and, for a session started with pushes, new
//!   items are written down it as they come;
//! - a **wait**, from a hook the program runs while its session is idle, answered when
//!   there is something to wake the session for and not before.
//!
//! They are counted apart from calls (`MAX_HELD`, not `MAX_CALLS`), each has a thread of
//! its own, and nothing is written to one while the journal is locked: a session that is
//! slow to read holds up nobody else.
use super::*;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};

pub(super) struct Watcher {
    id: u64,
    desk: String,
    waiting: bool,
    wake: Sender<()>,
}

/// How often a held connection looks whether the other end is still there.
const LOOK: Duration = Duration::from_secs(1);
/// How long a watch may be silent before a word is sent, so each end knows of the other.
const QUIET: Duration = Duration::from_secs(20);

/// Whether the other end of a held connection has gone. It never has anything to say
/// once held, so the end of its stream is the only thing there is to read.
fn gone(stream: &mut TcpStream) -> bool {
    let mut some = [0u8; 64];
    match stream.read(&mut some) {
        Ok(0) => true,
        Ok(_) => false,
        Err(e) => !matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut),
    }
}

impl Hub {
    /// Something has changed for a session: whatever is held open for it looks again now.
    pub(super) fn wake(&self, desk: &str) {
        for watcher in self.watchers.lock().unwrap().iter().filter(|w| w.desk == desk) {
            let _ = watcher.wake.send(());
        }
    }

    pub(super) fn wake_all(&self) {
        for watcher in self.watchers.lock().unwrap().iter() {
            let _ = watcher.wake.send(());
        }
    }

    fn enter(&self, desk: &str, waiting: bool) -> (u64, Receiver<()>) {
        let (wake, woken) = channel();
        let id = self.serial.fetch_add(1, Ordering::Relaxed) + 1;
        let mut watchers = self.watchers.lock().unwrap();
        // One wait for a session at a time. A newer one takes the place of an older, which
        // finds itself let go of and ends without a word.
        if waiting {
            watchers.retain(|w| !(w.waiting && w.desk == desk));
        }
        watchers.push(Watcher { id, desk: desk.to_string(), waiting, wake });
        (id, woken)
    }

    fn leave(&self, id: u64) {
        self.watchers.lock().unwrap().retain(|w| w.id != id);
    }

    /// Still the session's own, by the capability it came with.
    fn still(&self, desk: &str, token: &str) -> bool {
        !self.stopping.load(Ordering::Relaxed) && self.state.lock().unwrap().grants.get(token).map(String::as_str) == Some(desk)
    }

    pub(super) fn hold(&self, handle: &Handle, stream: TcpStream, desk: &str, token: &str, request: &Value) -> io::Result<()> {
        match request.get("watch") {
            Some(asked) => self.watch(handle, stream, desk, token, asked),
            None => self.wait(handle, stream, desk, token),
        }
    }

    fn watch(&self, handle: &Handle, mut stream: TcpStream, desk: &str, token: &str, asked: &Value) -> io::Result<()> {
        let (id, woken) = self.enter(desk, false);
        let said = |key: &str| crate::harness::shorten(asked[key].as_str().unwrap_or(""), 80);
        // What the program said of itself as it connected. It is pushed to only if this
        // launch was given pushes and its subprocess says it is passing them on.
        let (run, pushes) = {
            let mut state = self.state.lock().unwrap();
            let link = state.links.entry(desk.to_string()).or_default();
            link.live += 1;
            (link.client, link.protocol) = (said("client"), said("protocol"));
            (link.run, link.channel && asked["push"] == "channel")
        };
        let result = self.watching(handle, &mut stream, (desk, token), asked["after"].as_u64().unwrap_or(0), pushes, &woken);
        self.leave(id);
        // Unless the session has been started again meanwhile: that run's count is its own.
        if let Some(link) = self.state.lock().unwrap().links.get_mut(desk).filter(|link| link.run == run) {
            link.live = link.live.saturating_sub(1);
        }
        result
    }

    /// `whose` is the session and the capability it came with.
    fn watching(&self, handle: &Handle, stream: &mut TcpStream, whose: (&str, &str), after: u64, pushes: bool, woken: &Receiver<()>) -> io::Result<()> {
        let (desk, token) = whose;
        protocol::write_frame(stream, &json!({"watching":true}))?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        let (mut cursor, mut first, mut spoke) = (after, true, Instant::now());
        loop {
            if !self.still(desk, token) {
                return Ok(());
            }
            if pushes {
                for (revision, event) in self.push(handle, desk, cursor, first) {
                    protocol::write_frame(stream, &json!({"event":event}))?;
                    cursor = cursor.max(revision);
                    spoke = Instant::now();
                }
                first = false;
            }
            if gone(stream) {
                return Ok(());
            }
            match woken.recv_timeout(LOOK) {
                Ok(()) => while woken.try_recv().is_ok() {},
                Err(RecvTimeoutError::Timeout) => {
                    if spoke.elapsed() >= QUIET {
                        protocol::write_frame(stream, &json!({"ping":true}))?;
                        spoke = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    /// What to push to a session now. A program that has only just connected is told of
    /// everything unread, since what its last run was told may never have been read; after
    /// that, only of what nobody has told it. Each is recorded as told before it is
    /// written: if the writing fails, or the program was not taking pushes after all,
    /// another way says it again once the push has had its time.
    fn push(&self, handle: &Handle, desk: &str, cursor: u64, first: bool) -> Vec<(u64, Value)> {
        let now = now_ms();
        let due: Vec<Waiting> = {
            let mut state = self.state.lock().unwrap();
            let due: Vec<Waiting> = state.journal.waiting(desk).into_iter().filter(|w| w.revision > cursor && if first { w.delivery.unread(w.revision) } else { w.delivery.unannounced(w.revision) }).collect();
            if due.is_empty() {
                return Vec::new();
            }
            let mut journal = state.journal.clone();
            for item in &due {
                if let Some((_, delivery)) = journal.delivery_mut(desk, item.kind, &item.id) {
                    delivery.tell(item.revision, "channel", now);
                }
            }
            if self.commit(&mut state, journal).is_err() {
                return Vec::new();
            }
            due
        };
        let name = self.namer(handle);
        due.iter().map(|item| (item.revision, json!({"revision":item.revision,"kind":item.kind.word(),"id":item.id,"channel":mailbox::pushed(item, &name)}))).collect()
    }

    /// Held until there is something to wake an idle session for, then answered with it.
    /// Let go of without a word when a newer wait takes its place, the session ends, or
    /// the hook that asked has gone.
    fn wait(&self, handle: &Handle, mut stream: TcpStream, desk: &str, token: &str) -> io::Result<()> {
        let (id, woken) = self.enter(desk, true);
        if let Some(link) = self.state.lock().unwrap().links.get_mut(desk) {
            (link.hook_seen_ms, link.hook_seen) = (now_ms(), "wait".into());
        }
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        let context = loop {
            if !self.still(desk, token) {
                break None;
            }
            if gone(&mut stream) {
                self.leave(id);
                return Ok(());
            }
            // Not before its program has connected: woken with no tools to read anything
            // with, a session can only say that it has none.
            if self.live(desk) {
                let due = self.announce(desk, "wake");
                if !due.is_empty() {
                    break Some(mailbox::notice(&due, &self.namer(handle)));
                }
            }
            match woken.recv_timeout(LOOK) {
                Ok(()) => while woken.try_recv().is_ok() {},
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break None,
            }
        };
        self.leave(id);
        protocol::write_frame(&mut stream, &json!({"value":{"context":context}}))
    }
}
