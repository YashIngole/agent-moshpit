//! The local engine, as the app sees it: a speech worker process (see `worker`) started when
//! a recording needs it and kept for a few minutes after, so the next recording starts at once.
pub mod protocol;
pub mod worker;

use super::session::{Event, Session};
use protocol::{FromWorker, ToWorker, PROTOCOL};
use std::io::BufWriter;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long the worker (and the model it holds, about 1 GB) is kept after its last recording.
const KEEP: Duration = Duration::from_secs(5 * 60);
/// The engine's lookahead, in 80 ms steps. At 1 it types words every 0.16 s, the first about
/// 0.6 to 0.9 s into a sentence, using about 1.3 cores of a six-core laptop. At 6 it costs half
/// that, but words come in 0.56 s jumps and 0.3 s later (docs/research/voice-engines-2026-10.md).
const RIGHT_CONTEXT: i32 = 1;
const STOPPED: &str = "The local speech engine stopped unexpectedly. Words already typed stay. Try again.";

/// Engine threads: four at most, so agents' programs keep the rest of the processor.
fn threads() -> i32 {
    std::thread::available_parallelism().map_or(4, |n| n.get() / 2).clamp(1, 4) as i32
}

/// Where the worker's messages go: the recording using it now, by its id.
type Route = Arc<Mutex<Option<(u64, Sender<Event>)>>>;

struct Worker {
    child: Child,
    input: BufWriter<ChildStdin>,
    route: Route,
    alive: Arc<AtomicBool>,
    model: String,
    _tree: Option<crate::proctree::Tree>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = protocol::send(&mut self.input, &ToWorker::Shutdown);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
struct Slot {
    worker: Option<Worker>,
    /// Recordings that used the worker, so a later one keeps it from being let go.
    uses: u64,
    /// When the worker last ended without being asked to.
    crashes: Vec<Instant>,
    /// The last recording's id.
    recordings: u64,
}

#[derive(Clone, Default)]
pub struct Local {
    slot: Arc<Mutex<Slot>>,
}

impl Local {
    /// A recording on the local engine with this model file, the worker started if need be.
    pub fn open(&self, model: &Path) -> Result<Box<dyn Session>, String> {
        let mut slot = self.slot.lock().unwrap();
        slot.crashes.retain(|at| at.elapsed() < Duration::from_secs(600));
        if slot.worker.as_ref().is_some_and(|w| !w.alive.load(Ordering::Acquire)) {
            slot.worker = None;
            slot.crashes.push(Instant::now());
        }
        if slot.crashes.len() >= 3 {
            return Err("The local speech engine keeps stopping on this computer. Restart Agent Moshpit, or use a cloud provider in Voice input.".into());
        }
        let path = model.to_string_lossy().into_owned();
        if slot.worker.is_none() {
            slot.worker = Some(spawn()?);
        }
        slot.recordings += 1;
        let id = slot.recordings;
        let worker = slot.worker.as_mut().unwrap();
        let (sender, events) = channel();
        *worker.route.lock().unwrap() = Some((id, sender.clone()));
        let sent = protocol::send(&mut worker.input, &ToWorker::Load { path: path.clone(), threads: threads(), right_context: RIGHT_CONTEXT })
            .and_then(|()| protocol::send(&mut worker.input, &ToWorker::Begin { id }));
        if sent.is_err() {
            slot.worker = None;
            return Err(STOPPED.into());
        }
        worker.model = path;
        slot.uses += 1;
        Ok(Box::new(LocalSession { local: self.clone(), events, sender, finished: false, ended: false }))
    }

    /// Lets the worker go once no recording has used it for a while.
    fn release_later(&self) {
        let uses = self.slot.lock().unwrap().uses;
        let local = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(KEEP);
            let mut slot = local.slot.lock().unwrap();
            if slot.uses == uses {
                slot.worker = None;
            }
        });
    }

    /// Ends the worker now: the app is quitting, or its model is being removed.
    pub fn shutdown(&self) {
        self.slot.lock().unwrap().worker = None;
    }

    fn send(&self, message: &ToWorker) {
        if let Some(worker) = self.slot.lock().unwrap().worker.as_mut() {
            let _ = protocol::send(&mut worker.input, message);
        }
    }

    fn send_audio(&self, pcm: &[f32]) {
        if let Some(worker) = self.slot.lock().unwrap().worker.as_mut() {
            // A second at a time keeps every frame small; the worker joins them up again.
            for chunk in pcm.chunks(super::audio::RATE) {
                if protocol::send_audio(&mut worker.input, chunk).is_err() {
                    return;
                }
            }
        }
    }

    fn release_route(&self) {
        if let Some(worker) = self.slot.lock().unwrap().worker.as_ref() {
            *worker.route.lock().unwrap() = None;
        }
    }
}

/// Starts the app's own executable as a speech worker: no window, ended with the app.
fn spawn() -> Result<Worker, String> {
    let exe = std::env::current_exe().map_err(|e| format!("The local speech engine could not start: {e}"))?;
    let mut command = Command::new(exe);
    command.arg("--voice-worker").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|e| format!("The local speech engine could not start: {e}"))?;
    let tree = crate::proctree::Tree::new();
    if let Some(tree) = &tree {
        tree.add(child.id());
    }
    let input = BufWriter::new(child.stdin.take().ok_or(STOPPED)?);
    let mut output = std::io::BufReader::new(child.stdout.take().ok_or(STOPPED)?);
    let route: Route = Arc::default();
    let alive = Arc::new(AtomicBool::new(true));
    let (reader_route, reader_alive) = (route.clone(), alive.clone());
    std::thread::spawn(move || {
        // Said about one recording: only that recording hears it. Id 0 is about the worker.
        let tell = |id: u64, event: Event| {
            if let Some((current, sender)) = reader_route.lock().unwrap().as_ref() {
                if id == 0 || id == *current {
                    let _ = sender.send(event);
                }
            }
        };
        // Audio never comes this way; anything but a message means the worker is gone.
        while let Ok(Some(protocol::Frame::Message(message))) = protocol::receive::<FromWorker>(&mut output) {
            match message {
                    FromWorker::Hello { protocol, version } => {
                        // An app updated in place starts its new worker; this one is from another version.
                        if protocol != PROTOCOL || version != env!("CARGO_PKG_VERSION") {
                            tell(0, Event::Fault("Agent Moshpit was updated while it was running. Quit it and open it again to use local voice.".into()));
                            break;
                        }
                    }
                    FromWorker::Ready { .. } => tell(0, Event::Ready),
                    FromWorker::Text { id, committed, tentative } => {
                        if !committed.is_empty() {
                            tell(id, Event::Committed { text: committed, boundary: false });
                        }
                        tell(id, Event::Tentative(tentative));
                    }
                    FromWorker::Final { id, committed } => {
                        if !committed.is_empty() {
                            tell(id, Event::Committed { text: committed, boundary: false });
                        }
                        tell(id, Event::Tentative(String::new()));
                        tell(id, Event::Final);
                    }
                    FromWorker::Error { id, message } => tell(id, Event::Fault(message)),
            }
        }
        reader_alive.store(false, Ordering::Release);
        tell(0, Event::Fault(STOPPED.into()));
    });
    Ok(Worker { child, input, route, alive, model: String::new(), _tree: tree })
}

struct LocalSession {
    local: Local,
    events: Receiver<Event>,
    /// Kept so the channel stays open between messages.
    #[allow(dead_code)]
    sender: Sender<Event>,
    finished: bool,
    ended: bool,
}

impl Session for LocalSession {
    fn feed(&mut self, pcm: &[f32]) {
        if !self.ended && !pcm.is_empty() {
            self.local.send_audio(pcm);
        }
    }
    fn finish(&mut self) {
        if !self.ended && !self.finished {
            self.finished = true;
            self.local.send(&ToWorker::Finish);
        }
    }
    fn cancel(&mut self) {
        if !self.ended {
            self.ended = true;
            self.local.send(&ToWorker::Cancel);
        }
    }
    fn events(&self) -> &Receiver<Event> {
        &self.events
    }
}

impl Drop for LocalSession {
    fn drop(&mut self) {
        // A recording that ends any other way than finishing is dropped by the worker too.
        if !self.finished {
            self.cancel();
        }
        self.local.release_route();
        self.local.release_later();
    }
}
