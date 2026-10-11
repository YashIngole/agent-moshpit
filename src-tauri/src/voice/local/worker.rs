//! The speech worker: the app's own executable started with `--voice-worker`. It loads the
//! local model and turns audio into text, in a process of its own, so a fault in the native
//! engine ends this process and never the office (and with it every agent's program).
use super::protocol::{self, Frame, FromWorker, ToWorker, PROTOCOL};
use std::io::Write;
use std::sync::mpsc;
use std::time::Instant;
use transcribe_cpp::{Model, ParakeetStreamOptions, RunOptions, Session, SessionOptions, StreamExtension, StreamOptions};

/// What the worker received, from the thread that reads its stdin.
enum Input {
    Message(ToWorker),
    Audio(Vec<f32>),
    /// The app closed the pipe, went away, or sent something malformed.
    Gone,
}

/// Runs the worker until the app tells it to stop or goes away. Returns the exit code.
pub fn run() -> i32 {
    if std::env::args().any(|arg| arg == "--self-test") {
        // The release check: the worker starts, and says what it was built with.
        println!("agent-moshpit voice worker {} protocol {PROTOCOL} transcribe {}", env!("CARGO_PKG_VERSION"), transcribe_cpp::version());
        return 0;
    }
    let mut out = std::io::stdout().lock();
    if let Some(problem) = crate::voice::processor_problem() {
        let _ = protocol::send(&mut out, &FromWorker::Error { id: 0, message: problem.into() });
        return 3;
    }
    if std::env::var_os("MOSHPIT_DIAGNOSTICS").is_none() {
        transcribe_cpp::disable_logging();
    }
    if protocol::send(&mut out, &FromWorker::Hello { protocol: PROTOCOL, version: env!("CARGO_PKG_VERSION").into() }).is_err() {
        return 1;
    }
    let (sender, inputs) = mpsc::channel();
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        loop {
            let next = match protocol::receive::<ToWorker>(&mut input) {
                Ok(Some(Frame::Message(message))) => Input::Message(message),
                Ok(Some(Frame::Audio(pcm))) => Input::Audio(pcm),
                Ok(None) | Err(_) => Input::Gone,
            };
            let gone = matches!(next, Input::Gone);
            if sender.send(next).is_err() || gone {
                return;
            }
        }
    });
    let mut loaded: Option<(String, Model)> = None;
    let mut session: Option<Session> = None;
    let mut options = (4, 6);
    loop {
        match inputs.recv().unwrap_or(Input::Gone) {
            Input::Message(ToWorker::Load { path, threads, right_context }) => {
                options = (threads, right_context);
                if loaded.as_ref().is_some_and(|(p, _)| *p == path) {
                    let _ = protocol::send(&mut out, &FromWorker::Ready { load_ms: 0 });
                    continue;
                }
                session = None;
                loaded = None;
                let began = Instant::now();
                match Model::load(&path) {
                    Ok(model) if model.capabilities().supports_streaming => {
                        session = model.session_with(&SessionOptions { n_threads: threads, ..Default::default() }).ok();
                        loaded = Some((path, model));
                        let message = match session {
                            Some(_) => FromWorker::Ready { load_ms: began.elapsed().as_millis() as u64 },
                            None => FromWorker::Error { id: 0, message: "The local speech model could not start. Check that there is free memory, then try again.".into() },
                        };
                        let _ = protocol::send(&mut out, &message);
                    }
                    Ok(_) => {
                        let _ = protocol::send(&mut out, &FromWorker::Error { id: 0, message: "That model cannot transcribe as you speak. Choose another one in Voice input.".into() });
                    }
                    Err(e) => {
                        let _ = protocol::send(&mut out, &FromWorker::Error { id: 0, message: format!("The local speech model could not load: {e}. Remove it in Voice input and download it again.") });
                    }
                }
            }
            Input::Message(ToWorker::Begin { id }) => match session.as_mut() {
                Some(session) => {
                    if !dictate(id, session, options.1, &inputs, &mut out) {
                        return 0;
                    }
                }
                None => {
                    let _ = protocol::send(&mut out, &FromWorker::Error { id, message: "The local speech model is not loaded.".into() });
                }
            },
            // Audio or an ending for a recording that was already over: nothing to do.
            Input::Message(ToWorker::Finish | ToWorker::Cancel) | Input::Audio(_) => {}
            Input::Message(ToWorker::Shutdown) | Input::Gone => return 0,
        }
    }
}

/// One recording. Settled text is sent as it grows, with the text still being decided after
/// it. False when the app has gone and the worker should end.
fn dictate(id: u64, session: &mut Session, right_context: i32, inputs: &mpsc::Receiver<Input>, out: &mut impl Write) -> bool {
    let options = StreamOptions { family: Some(StreamExtension::ParakeetStream(ParakeetStreamOptions { att_context_right: Some(right_context) })), ..Default::default() };
    let mut stream = match session.stream(&RunOptions::default(), &options) {
        Ok(stream) => stream,
        Err(e) => {
            let _ = protocol::send(out, &FromWorker::Error { id, message: format!("The local speech engine could not start listening: {e}.") });
            return true;
        }
    };
    let mut sent = 0usize;
    let mut tentative = String::new();
    // What is new in the settled text since it was last sent. It only ever grows; should the
    // engine rewrite it, the rewrite is not typed again.
    let delta = |committed: &str, sent: &mut usize| -> String {
        let new = committed.get(*sent..).unwrap_or_default().to_string();
        *sent = committed.len().max(*sent);
        new
    };
    // A message that arrived behind audio, kept for the next turn.
    let mut held: Option<Input> = None;
    loop {
        match held.take().unwrap_or_else(|| inputs.recv().unwrap_or(Input::Gone)) {
            Input::Audio(mut pcm) => {
                // Everything already waiting goes in at once, so a slow moment is caught up.
                loop {
                    match inputs.try_recv() {
                        Ok(Input::Audio(more)) => pcm.extend_from_slice(&more),
                        Ok(other) => {
                            held = Some(other);
                            break;
                        }
                        Err(_) => break,
                    }
                }
                let update = match stream.feed(&pcm) {
                    Ok(update) => update,
                    Err(e) => {
                        let _ = protocol::send(out, &FromWorker::Error { id, message: format!("The local speech engine stopped: {e}.") });
                        return true;
                    }
                };
                if update.committed_changed || update.tentative_changed {
                    let text = stream.text();
                    let committed = delta(&text.committed, &mut sent);
                    if !committed.is_empty() || text.tentative != tentative {
                        tentative = text.tentative.clone();
                        if protocol::send(out, &FromWorker::Text { id, committed, tentative: text.tentative }).is_err() {
                            return false;
                        }
                    }
                }
            }
            Input::Message(ToWorker::Finish) => {
                let message = match stream.finalize() {
                    Ok(_) => FromWorker::Final { id, committed: delta(&stream.text().committed, &mut sent) },
                    Err(e) => FromWorker::Error { id, message: format!("The local speech engine could not finish: {e}.") },
                };
                return protocol::send(out, &message).is_ok();
            }
            Input::Message(ToWorker::Cancel) => return true,
            Input::Message(ToWorker::Begin { .. } | ToWorker::Load { .. }) => {
                let _ = protocol::send(out, &FromWorker::Error { id, message: "The local speech engine was asked to start twice.".into() });
                return true;
            }
            Input::Message(ToWorker::Shutdown) | Input::Gone => return false,
        }
    }
}
