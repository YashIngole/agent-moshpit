//! Dictation into a terminal. A recording owns one terminal run, types words as they are
//! settled, and never submits a prompt. The words come from a speech engine (see `session`):
//! the local one on this computer (`local`), or a cloud provider with the user's own key
//! (`cloud`).
mod audio;
mod cloud;
mod keys;
mod local;
mod models;
mod segment;
mod session;
pub use audio::{inputs, Inputs};

use crate::engine::Handle;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use session::{Event, Joiner, Session};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Runs the speech worker (`--voice-worker`). Returns the process's exit code.
pub fn run_worker() -> i32 {
    local::worker::run()
}

/// Which engine turns speech into words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// On this computer: nothing is sent anywhere.
    #[default]
    Local,
    Deepgram,
    Assemblyai,
    Openai,
    Elevenlabs,
    Openrouter,
}

impl Engine {
    fn provider(self) -> Option<cloud::Provider> {
        match self {
            Engine::Local => None,
            Engine::Deepgram => Some(cloud::Provider::Deepgram),
            Engine::Assemblyai => Some(cloud::Provider::Assemblyai),
            Engine::Openai => Some(cloud::Provider::Openai),
            Engine::Elevenlabs => Some(cloud::Provider::Elevenlabs),
            Engine::Openrouter => Some(cloud::Provider::Openrouter),
        }
    }
}

/// A setting that cannot be read takes its default; the others are kept. One unknown value
/// (say, written by a newer version) must not reset everything else the user chose.
pub fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(d: D) -> Result<T, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
fn lenient_yes<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or(true))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "lenient")]
    pub enabled: bool,
    #[serde(deserialize_with = "lenient")]
    pub engine: Engine,
    /// The local model's id (`models::MODELS`).
    #[serde(deserialize_with = "lenient")]
    pub local_model: String,
    #[serde(deserialize_with = "lenient")]
    pub shortcut: Shortcut,
    #[serde(deserialize_with = "lenient")]
    pub microphone: Option<String>,
    /// The user's own words for a cloud provider to expect: names, jargon.
    #[serde(deserialize_with = "lenient")]
    pub vocabulary: Vec<String>,
    /// A cloud provider is also told the desk's name, folder and branch.
    #[serde(deserialize_with = "lenient_yes")]
    pub send_desk_names: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: Engine::Local,
            local_model: models::DEFAULT.into(),
            shortcut: Shortcut::Space,
            microphone: None,
            vocabulary: Vec::new(),
            send_desk_names: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shortcut {
    #[default]
    Space,
    Altspace,
    None,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    #[default]
    Idle,
    Preparing,
    Listening,
    Transcribing,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub agent: String,
    pub run: u64,
}

/// The "desk" of a recording whose words go to a field in the window (New agent's task),
/// not to a terminal. No desk has this for an id.
const FIELD: &str = "\u{0}field";

/// Where the words of such a recording go: a piece of text, and whether a space stands
/// between it and what went before. False once the window no longer takes them.
pub type FieldSink = Box<dyn Fn(&str, bool) -> bool + Send>;

impl Target {
    fn is_field(&self) -> bool {
        self.agent == FIELD
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelView {
    pub id: &'static str,
    pub label: &'static str,
    pub bytes: u64,
    pub ready: bool,
    pub recommended: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProviderView {
    pub id: &'static str,
    pub name: &'static str,
    /// A key is kept for it, in the keychain or for this session.
    pub configured: bool,
    pub last4: String,
    pub session_only: bool,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct View {
    pub sequence: u64,
    pub phase: Phase,
    pub agent: Option<String>,
    pub started_ms: Option<u64>,
    pub message: String,
    /// The engine of the recording shown.
    pub engine: Engine,
    /// What the engine is doing before it hears: loading, connecting. Empty once it hears.
    pub detail: String,
    pub models: Vec<ModelView>,
    pub downloading: Option<String>,
    pub received: u64,
    pub download_error: String,
    pub verifying: bool,
    pub busy: bool,
    pub level: u32,
    pub microphone: String,
    pub transcribing_ms: Option<u64>,
    /// The words still being decided, after those already typed.
    pub partial: String,
    /// The words of the recording shown go to a field in the window, not to a terminal.
    pub field: bool,
    /// Why this computer cannot run the local engine; empty when it can. Cloud engines need nothing.
    pub unsupported: String,
    pub providers: Vec<ProviderView>,
}

#[derive(Default)]
struct Machine {
    view: View,
    epoch: u64,
    target: Option<Target>,
    /// Where the words go when the recording is for a field in the window.
    field: Option<FieldSink>,
    // 0 captures, 1 stops and settles, 2 cancels and discards.
    mode: Arc<AtomicU8>,
    /// Local models whose files passed their check.
    ready: Vec<&'static str>,
    download_cancel: Option<Arc<AtomicBool>>,
    window_open: bool,
    /// Tells the window something changed (see `Voice::new`).
    publish: Option<SyncSender<()>>,
}
impl Machine {
    fn changed(&mut self) {
        self.view.sequence += 1;
        if let Some(publish) = &self.publish {
            let _ = publish.try_send(());
        }
    }
    fn accepts(&self, epoch: u64, target: &Target) -> bool {
        self.epoch == epoch && self.target.as_ref() == Some(target) && self.mode.load(Ordering::Acquire) != 2
    }
    fn cancel(&mut self) {
        self.mode.store(2, Ordering::Release);
        self.epoch += 1;
        self.target = None;
        self.field = None;
        self.view.field = false;
        self.view.phase = Phase::Idle;
        self.view.agent = None;
        self.view.started_ms = None;
        self.view.message.clear();
        self.view.detail.clear();
        self.view.level = 0;
        self.view.microphone.clear();
        self.view.transcribing_ms = None;
        self.view.partial.clear();
        self.changed();
    }

    /// Stop listening and finish. Also while the microphone is still opening: a shortcut held
    /// and released by then ends the recording before it began.
    fn stop(&mut self) -> Result<(), String> {
        if !matches!(self.view.phase, Phase::Listening | Phase::Preparing) {
            return Err("Voice is not listening.".into());
        }
        self.mode.store(1, Ordering::Release);
        self.view.phase = Phase::Transcribing;
        self.view.level = 0;
        self.view.transcribing_ms = Some(crate::model::now_ms());
        self.changed();
        Ok(())
    }
}

#[derive(Clone)]
pub struct Voice {
    state: Arc<Mutex<Machine>>,
    dir: PathBuf,
    // File checks, removal and promotion cannot race each other. A download writes its own
    // partial file and takes this only to move it into place.
    disk: Arc<Mutex<()>>,
    local: local::Local,
    keys: Arc<keys::Keys>,
}

impl Voice {
    /// `dir` holds the local models, `keys` the list of providers with a saved key. `notify`
    /// is given the view whenever it changes, at most about thirty times a second.
    pub fn new(dir: PathBuf, keys: PathBuf, notify: impl Fn(View) + Send + 'static) -> Self {
        let (publish, changes) = sync_channel(1);
        let voice = Self {
            state: Arc::new(Mutex::new(Machine { publish: Some(publish), ..Machine::default() })),
            dir,
            disk: Arc::new(Mutex::new(())),
            local: local::Local::default(),
            keys: Arc::new(keys::Keys::new(keys)),
        };
        {
            let mut state = voice.state.lock().unwrap();
            state.view.verifying = true;
            state.view.unsupported = processor_problem().unwrap_or_default().into();
        }
        let check = voice.clone();
        std::thread::spawn(move || {
            let _disk = check.disk.lock().unwrap();
            // Whisper's models, from before this version, are not used any more.
            for old in models::RETIRED {
                let _ = std::fs::remove_file(check.dir.join(old));
            }
            let ready: Vec<&'static str> = models::MODELS.iter().filter(|m| models::ready(&check.dir, m, &AtomicBool::new(false)).is_ok()).map(|m| m.id).collect();
            let mut state = check.state.lock().unwrap();
            state.ready = ready;
            state.view.verifying = false;
            state.changed();
        });
        // Changes come in bursts (the level meter moves fifty times a second): the window
        // gets the view as it is after each short quiet moment, and never under the lock.
        let publisher = voice.clone();
        std::thread::spawn(move || {
            while changes.recv().is_ok() {
                std::thread::sleep(Duration::from_millis(30));
                while changes.try_recv().is_ok() {}
                notify(publisher.view());
            }
        });
        voice
    }

    pub fn view(&self) -> View {
        let kept = self.keys.list();
        let state = self.state.lock().unwrap();
        View {
            models: models::MODELS
                .iter()
                .map(|m| ModelView { id: m.id, label: m.label, bytes: m.bytes, ready: state.ready.contains(&m.id), recommended: m.recommended })
                .collect(),
            providers: cloud::PROVIDERS
                .iter()
                .map(|p| {
                    let saved = kept.get(p.id());
                    ProviderView { id: p.id(), name: p.name(), configured: saved.is_some(), last4: saved.map(|k| k.last4.clone()).unwrap_or_default(), session_only: saved.is_some_and(|k| k.session_only) }
                })
                .collect(),
            ..state.view.clone()
        }
    }

    pub fn cancel(&self) {
        self.state.lock().unwrap().cancel();
    }
    pub fn window(&self, open: bool) {
        let mut state = self.state.lock().unwrap();
        // The window going away ends a recording. Further events while it stays
        // minimized change nothing, so they cannot clear a message shown later.
        if !open && state.window_open {
            state.cancel();
        }
        state.window_open = open;
    }
    pub fn cancel_agent(&self, agent: &str) {
        let mut state = self.state.lock().unwrap();
        if state.target.as_ref().is_some_and(|t| t.agent == agent) {
            state.cancel();
        }
    }
    pub fn shutdown(&self) {
        self.cancel();
        self.cancel_download();
        self.local.shutdown();
    }
    pub fn stop(&self) -> Result<(), String> {
        self.state.lock().unwrap().stop()
    }

    /// Record into a desk's terminal: its words are typed there as they are settled.
    pub fn start(&self, engine: Handle, agent: String, settings: Settings) -> Result<(), String> {
        self.begin(engine, settings, Some(agent), None)
    }

    /// Record into a field in the window: each settled piece is handed to `sink`, with
    /// whether a space stands before it. The same engine, the same rules about what a
    /// word may contain, and the same ends: stopped, cancelled, or out of time.
    pub fn start_field(&self, engine: Handle, settings: Settings, sink: FieldSink) -> Result<(), String> {
        self.begin(engine, settings, None, Some(sink))
    }

    fn begin(&self, engine: Handle, settings: Settings, agent: Option<String>, field: Option<FieldSink>) -> Result<(), String> {
        if !settings.enabled {
            return Err("Turn on voice input in the Voice input panel first.".into());
        }
        let model = models::find(&settings.local_model).unwrap_or(&models::MODELS[0]);
        match settings.engine.provider() {
            None => {
                if let Some(problem) = processor_problem() {
                    return Err(format!("{problem} Choose a cloud provider in Voice input instead."));
                }
            }
            Some(provider) => {
                if !self.keys.has(provider.id()) {
                    return Err(format!("Add your {} key in Voice input first.", provider.name()));
                }
            }
        }
        let target = match &agent {
            Some(agent) => engine.voice_target(agent).ok_or("Select a visible terminal whose program is running before recording.")?,
            None => Target { agent: FIELD.into(), run: crate::model::now_ms() },
        };
        let mut state = self.state.lock().unwrap();
        if !state.window_open {
            return Err("Open the office window before recording.".into());
        }
        if state.view.busy {
            return Err("The previous recording is finishing. Try again in a moment.".into());
        }
        if settings.engine == Engine::Local && !state.ready.contains(&model.id) {
            return Err("Download the local speech model in Voice input first, then try again.".into());
        }
        state.cancel();
        state.mode = Arc::new(AtomicU8::new(0));
        state.target = Some(target.clone());
        state.view.field = field.is_some();
        state.field = field;
        state.view.phase = Phase::Preparing;
        state.view.agent = agent;
        state.view.engine = settings.engine;
        state.view.detail = match settings.engine.provider() {
            None => "Loading the local speech model…".into(),
            Some(provider) => format!("Connecting to {}…", provider.name()),
        };
        state.view.busy = true;
        state.changed();
        let (epoch, mode) = (state.epoch, state.mode.clone());
        drop(state);
        let voice = self.clone();
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| voice.record(&engine, &target, epoch, &mode, &settings, model)));
            let mut state = voice.state.lock().unwrap();
            if state.accepts(epoch, &target) {
                match outcome {
                    Ok(Ok(note)) => {
                        state.view.phase = Phase::Idle;
                        state.view.message = note.unwrap_or_else(|| if target.is_field() { "Added to the task.".into() } else { "Inserted. Press Enter in the terminal to send.".into() });
                    }
                    Ok(Err(why)) => {
                        state.view.phase = Phase::Error;
                        state.view.message = why;
                    }
                    Err(_) => {
                        state.view.phase = Phase::Error;
                        state.view.message = "Voice input stopped unexpectedly. Try again.".into();
                    }
                }
                state.target = None;
                state.field = None;
                state.view.started_ms = None;
                state.view.level = 0;
                state.view.transcribing_ms = None;
                state.view.partial.clear();
                state.view.detail.clear();
            }
            state.view.busy = false;
            state.changed();
        });
        Ok(())
    }

    /// One recording, typed into its terminal as its words are settled. Ok carries a note to
    /// show instead of the usual one.
    fn record(&self, engine: &Handle, target: &Target, epoch: u64, mode: &Arc<AtomicU8>, settings: &Settings, model: &'static models::Model) -> Result<Option<String>, String> {
        // A program can end under a recording; a field is the window's, which cancels the
        // recording itself when the field goes. What the window shows meanwhile ends nothing.
        let live = || target.is_field() || engine.voice_live(target);
        if mode.load(Ordering::Acquire) == 2 || !live() {
            return Err("Recording cancelled.".into());
        }
        // The engine starts (the model loads, or the socket connects) while the microphone
        // opens and the user starts speaking; audio heard meanwhile is kept for it.
        let mut session: Box<dyn Session> = match settings.engine.provider() {
            None => {
                {
                    let _disk = self.disk.lock().unwrap();
                    models::ready(&self.dir, model, &AtomicBool::new(false))?;
                }
                self.local.open(&self.dir.join(model.file))?
            }
            Some(provider) => {
                let mut words = settings.vocabulary.clone();
                words.extend(engine.voice_context(&target.agent, settings.send_desk_names));
                let key = self.keys.get(provider.id())?;
                cloud::open(provider, key, &words)
            }
        };
        #[cfg(debug_assertions)]
        let fixture = fixture_pcm()?;
        #[cfg(not(debug_assertions))]
        let fixture: Option<Vec<f32>> = None;
        // An explicit debug fixture NEVER falls back to the microphone, even on failure.
        let capture = if fixture.is_some() { None } else { Some(audio::open(settings.microphone.as_deref())?) };
        {
            let mut state = self.state.lock().unwrap();
            if !state.accepts(epoch, target) {
                return Err("Recording cancelled.".into());
            }
            if mode.load(Ordering::Acquire) == 1 {
                return Err("The shortcut was released before the microphone opened. Hold it while you speak, or tap it once to talk hands-free.".into());
            }
            state.view.phase = Phase::Listening;
            state.view.started_ms = Some(crate::model::now_ms());
            state.view.microphone = capture.as_ref().map_or_else(|| "Recorded audio fixture".into(), |c| c.name.clone());
            state.changed();
        }
        let mut words = Words { voice: self, engine, target, epoch, joiner: Joiner::default(), tentative: String::new() };
        let mut segmenter = segment::Segmenter::default();
        // A fixture is heard all at once, or (`MOSHPIT_VOICE_WAV_PACED`) as fast as it was spoken.
        let paced = fixture.is_some() && cfg!(debug_assertions) && std::env::var_os("MOSHPIT_VOICE_WAV_PACED").is_some();
        let (mut heard, spoken) = if paced { (Vec::new(), fixture.unwrap_or_default()) } else { (fixture.unwrap_or_default(), Vec::new()) };
        let mut fed = 0;
        let began = Instant::now();
        let mut last_audio = Instant::now();
        while mode.load(Ordering::Acquire) == 0 && heard.len() < audio::MAX_SAMPLES && began.elapsed() < Duration::from_secs(audio::MAX_SECONDS as u64) {
            if !live() {
                self.cancel_agent(&target.agent);
                break;
            }
            if paced {
                let due = (began.elapsed().as_millis() as usize * audio::RATE / 1000).min(spoken.len());
                if due > heard.len() {
                    heard.extend_from_slice(&spoken[heard.len()..due]);
                }
            }
            if let Some(capture) = &capture {
                if capture.failed.load(Ordering::Acquire) {
                    break;
                }
                let before = heard.len();
                heard.extend_from_slice(&capture.audio.lock().unwrap().pcm[before..]);
                if heard.len() != before {
                    last_audio = Instant::now();
                } else if last_audio.elapsed() >= Duration::from_secs(3) {
                    return Err("The microphone is not delivering audio. Check microphone access in system settings, then choose an available input in Voice input.".into());
                }
                let level = capture.level.load(Ordering::Acquire);
                let mut state = self.state.lock().unwrap();
                if state.accepts(epoch, target) && state.view.level != level {
                    state.view.level = level;
                    state.changed();
                }
            }
            if heard.len() > fed {
                session.feed(&heard[fed..]);
                fed = heard.len();
            }
            if segmenter.step(&heard) {
                session.pause();
            }
            while let Ok(event) = session.events().try_recv() {
                words.take(event, false)?;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Stop capture first. The stream stays on its owner thread.
        let failed = match capture {
            Some(audio::DeviceCapture { stream, audio, failed, .. }) => {
                drop(stream);
                let before = heard.len();
                heard.extend_from_slice(&audio.lock().unwrap().pcm[before..]);
                failed.load(Ordering::Acquire)
            }
            None => false,
        };
        if mode.load(Ordering::Acquire) == 2 {
            session.cancel();
            return Err("Recording cancelled.".into());
        }
        if failed {
            session.cancel();
            return Err("The microphone stopped or lost audio. Check the input device and try again.".into());
        }
        if heard.len() > fed {
            session.feed(&heard[fed..]);
        }
        {
            let mut state = self.state.lock().unwrap();
            if !state.accepts(epoch, target) {
                session.cancel();
                return Err("Recording cancelled.".into());
            }
            state.view.phase = Phase::Transcribing;
            state.view.level = 0;
            state.view.transcribing_ms.get_or_insert_with(crate::model::now_ms);
            state.changed();
        }
        // A recording that was all room noise says so, unless words were already typed.
        if !words.joiner.has_typed() {
            if let Err(why) = audio::speech(&heard) {
                session.cancel();
                return Err(why);
            }
        }
        session.finish();
        // The engine settles the last words; usually a tenth of a second. One that is sent
        // each phrase whole has the last one still to hear back about.
        let deadline = Instant::now() + session.settle_within();
        let mut settled = false;
        while !settled {
            let left = deadline.saturating_duration_since(Instant::now());
            match session.events().recv_timeout(left) {
                Ok(event) => settled = words.take(event, true)?,
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        let mut note = None;
        if !settled && !words.tentative.trim().is_empty() {
            // The engine did not answer in time: what it last showed is typed, and said to be.
            let last = std::mem::take(&mut words.tentative);
            words.type_text(&last, true)?;
            note = Some(if target.is_field() { "Added the last words as they were shown live; check them.".into() } else { "Inserted the last words as they were shown live; check them before pressing Enter.".into() });
        }
        if words.joiner.has_typed() {
            Ok(note)
        } else {
            Err("No words were recognised. Try again, a little closer to the microphone.".into())
        }
    }

    fn show(&self, epoch: u64, target: &Target, change: impl FnOnce(&mut View) -> bool) {
        let mut state = self.state.lock().unwrap();
        if state.accepts(epoch, target) && change(&mut state.view) {
            state.changed();
        }
    }

    pub fn download(&self, id: &str) -> Result<(), String> {
        let model = models::find(id).ok_or("That model is not one Agent Moshpit offers.")?;
        let mut state = self.state.lock().unwrap();
        if state.view.downloading.is_some() || state.view.verifying {
            return Err("A model is being downloaded or checked. Wait for it to finish.".into());
        }
        if state.ready.contains(&model.id) {
            return Ok(());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        state.download_cancel = Some(cancelled.clone());
        state.view.downloading = Some(model.id.into());
        state.view.received = 0;
        state.view.download_error.clear();
        state.changed();
        let voice = self.clone();
        std::thread::spawn(move || {
            // Only this download writes its partial file, so recording with a model that is
            // already in place goes on meanwhile. The folder is locked only to move it in.
            let partial = voice.dir.join(format!("{}.part", model.file));
            let result = (|| {
                std::fs::create_dir_all(&voice.dir).map_err(|e| format!("The voice model folder could not be made: {e}"))?;
                let mut shown = Instant::now();
                models::download(&partial, model, &cancelled, |received| {
                    let mut state = voice.state.lock().unwrap();
                    state.view.received = received;
                    // Four times a second is enough for a progress bar.
                    if shown.elapsed() >= Duration::from_millis(250) {
                        shown = Instant::now();
                        state.changed();
                    }
                })
            })();
            let _disk = voice.disk.lock().unwrap();
            let mut state = voice.state.lock().unwrap();
            let result = if cancelled.load(Ordering::Acquire) { Err("Download cancelled.".into()) } else { result };
            match result.and_then(|verified| verified.promote(&voice.dir.join(model.file), model.sha256, &cancelled)) {
                Ok(()) => {
                    state.ready.retain(|m| *m != model.id);
                    state.ready.push(model.id);
                }
                // An interrupted download is kept, so Download carries on where it stopped.
                Err(why) => state.view.download_error = why,
            }
            state.view.downloading = None;
            state.download_cancel = None;
            state.changed();
        });
        Ok(())
    }
    pub fn cancel_download(&self) {
        if let Some(cancelled) = &self.state.lock().unwrap().download_cancel {
            cancelled.store(true, Ordering::Release);
        }
    }
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let model = models::find(id).ok_or("That model is not one Agent Moshpit offers.")?;
        self.cancel();
        // The worker holds the model open; it lets go before the file is taken away.
        self.local.shutdown();
        let _disk = self.disk.try_lock().map_err(|_| "A model is being checked or downloaded. Cancel it or wait, then try again.".to_string())?;
        let mut state = self.state.lock().unwrap();
        if state.view.busy {
            return Err("The recording is finishing. Try removing the model in a moment.".into());
        }
        models::remove(&self.dir, model)?;
        state.ready.retain(|m| *m != model.id);
        state.changed();
        Ok(())
    }

    /// Keeps a provider's key, checked with the provider first unless told not to.
    pub fn set_key(&self, provider: &str, key: &str, session_only: bool, check: bool) -> Result<(), String> {
        let provider = cloud::Provider::from_id(provider).ok_or("That provider is not one Agent Moshpit offers.")?;
        let key = key.trim();
        if key.len() < 8 || key.len() > 512 || key.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("That does not look like an API key. Copy it again from the provider's dashboard.".into());
        }
        if check {
            cloud::check(provider, key)?;
        }
        self.keys.set(provider.id(), key, session_only)?;
        self.state.lock().unwrap().changed();
        Ok(())
    }
    /// Checks the kept key with its provider.
    pub fn check_key(&self, provider: &str) -> Result<(), String> {
        let provider = cloud::Provider::from_id(provider).ok_or("That provider is not one Agent Moshpit offers.")?;
        cloud::check(provider, &self.keys.get(provider.id())?)
    }
    pub fn remove_key(&self, provider: &str) -> Result<(), String> {
        let provider = cloud::Provider::from_id(provider).ok_or("That provider is not one Agent Moshpit offers.")?;
        let result = self.keys.remove(provider.id());
        self.state.lock().unwrap().changed();
        result
    }
}

/// The words of one recording: settled ones typed into its terminal, the rest shown.
struct Words<'a> {
    voice: &'a Voice,
    engine: &'a Handle,
    target: &'a Target,
    epoch: u64,
    joiner: Joiner,
    tentative: String,
}

impl Words<'_> {
    /// Acts on what the engine said. True once everything is settled after stopping.
    fn take(&mut self, event: Event, stopped: bool) -> Result<bool, String> {
        match event {
            Event::Ready => self.voice.show(self.epoch, self.target, |view| !std::mem::take(&mut view.detail).is_empty()),
            Event::Committed { text, boundary } => self.type_text(&text, boundary)?,
            Event::Tentative(text) => {
                self.tentative = text.clone();
                let shown = text.trim().to_string();
                self.voice.show(self.epoch, self.target, |view| view.partial != shown && { view.partial = shown; true });
            }
            Event::Final => return Ok(stopped),
            Event::Fault(why) => return Err(why),
        }
        Ok(false)
    }

    fn type_text(&mut self, text: &str, boundary: bool) -> Result<(), String> {
        let Some((piece, space)) = self.joiner.next(text, boundary) else { return Ok(()) };
        let state = self.voice.state.lock().unwrap();
        if !state.accepts(self.epoch, self.target) {
            return Err("Recording cancelled.".into());
        }
        if self.target.is_field() {
            // The same rules about what a word may hold as for a terminal: one line, and
            // nothing that is not text.
            let text = sanitize(&piece);
            if text.is_empty() || text.len() > 32 * 1024 {
                return Err("The voice text was empty or too long and discarded.".into());
            }
            if !state.field.as_ref().is_some_and(|sink| sink(&text, space)) {
                return Err("The form was closed before the recording ended. What was already added stays.".into());
            }
        } else {
            // The same text path as terminal keys. PTY generation is checked under the write
            // lock; a restart cannot redirect text into its replacement.
            self.engine.voice_write(self.target, piece.as_bytes(), space)?;
        }
        self.joiner.typed();
        Ok(())
    }
}

#[cfg(debug_assertions)]
fn fixture_pcm() -> Result<Option<Vec<f32>>, String> {
    let Some(path) = std::env::var_os("MOSHPIT_VOICE_WAV") else {
        return Ok(None);
    };
    if crate::instance_name().is_none() || std::env::var_os("MOSHPIT_DATA_DIR").is_none() {
        return Err("Voice WAV fixtures require a named instance and isolated MOSHPIT_DATA_DIR. Microphone was not opened.".into());
    }
    let mut wav = hound::WavReader::open(path).map_err(|e| format!("The voice WAV fixture could not load: {e}. Microphone was not opened."))?;
    let spec = wav.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 || wav.duration() > spec.sample_rate * audio::MAX_SECONDS as u32 {
        return Err(format!("Use a PCM16 WAV fixture no longer than {} seconds. Microphone was not opened.", audio::MAX_SECONDS));
    }
    let mut capture = audio::Capture::new(spec.channels, spec.sample_rate)?;
    let mut chunk = Vec::with_capacity(4096);
    for sample in wav.samples::<i16>() {
        chunk.push(sample.map_err(|e| e.to_string())?);
        if chunk.len() == 4096 {
            capture.push(&chunk);
            chunk.clear();
        }
    }
    capture.push(&chunk);
    Ok(Some(capture.pcm))
}

/// Why this computer cannot run the local engine, if it cannot. On x86-64 it is built for
/// AVX2, FMA, F16C and BMI2 (.cargo/config.toml) and would crash on a processor without them,
/// so the worker is never started there. Cloud engines run on any processor.
pub fn processor_problem() -> Option<&'static str> {
    #[cfg(target_arch = "x86_64")]
    if !(std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("fma")
        && std::arch::is_x86_feature_detected!("f16c")
        && std::arch::is_x86_feature_detected!("bmi2"))
    {
        return Some("The local speech engine needs a processor with AVX2 (Intel from 2013 or AMD from 2015), and this computer's processor does not have it.");
    }
    None
}

/// Remove ANSI/C1 CSI and strings (OSC/DCS/etc), all controls, and line breaks.
/// Only a single plain-text line can reach the PTY; it contains no Enter.
pub fn sanitize(text: &str) -> String {
    #[derive(Clone, Copy)]
    enum Escape {
        Plain,
        Esc,
        Csi,
        String,
        StringEsc,
    }
    let mut mode = Escape::Plain;
    let mut clean = String::new();
    for c in text.chars() {
        match mode {
            Escape::Plain => match c {
                '\u{1b}' => mode = Escape::Esc,
                '\u{9b}' => mode = Escape::Csi,
                '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => mode = Escape::String,
                c if c.is_whitespace() => clean.push(' '),
                c if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') => {}
                c => clean.push(c),
            },
            Escape::Esc => {
                mode = match c {
                    '[' => Escape::Csi,
                    ']' | 'P' | 'X' | '^' | '_' => Escape::String,
                    '\u{20}'..='\u{2f}' => Escape::Esc,
                    _ => Escape::Plain,
                }
            }
            Escape::Csi => {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    mode = Escape::Plain;
                }
            }
            Escape::String => match c {
                '\u{7}' | '\u{9c}' => mode = Escape::Plain,
                '\u{1b}' => mode = Escape::StringEsc,
                _ => {}
            },
            Escape::StringEsc => mode = if c == '\\' { Escape::Plain } else { Escape::String },
        }
    }
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_sequences_and_enter_never_survive() {
        assert_eq!(sanitize(" fix\r\nfile\t\x1b[31mred\x1b[0m\x00\x03\x7f नमस्ते "), "fix file red नमस्ते");
        assert_eq!(sanitize("a\x1b]52;c;clipboard\x07b\x1bPcommand\x1b\\c"), "abc");
        assert_eq!(sanitize("a\u{9b}31mb\u{9d}secret\u{9c}c"), "abc");
        assert_eq!(sanitize("safe\x1b]unterminated"), "safe");
    }
    #[test]
    fn cancel_and_new_generation_reject_stale_results() {
        let mut state = Machine::default();
        let target = Target { agent: "desk".into(), run: 1 };
        state.target = Some(target.clone());
        state.view.phase = Phase::Listening;
        assert!(state.accepts(0, &target));
        assert!(!state.accepts(0, &Target { run: 2, ..target.clone() }));
        state.cancel();
        assert!(!state.accepts(0, &target));
        state.mode = Arc::new(AtomicU8::new(0));
        state.target = Some(target.clone());
        assert!(!state.accepts(0, &target));
        assert!(state.accepts(1, &target));
    }

    struct NoDesktop;
    impl crate::engine::Shell for NoDesktop {
        fn snapshot(&self, _: &crate::model::Snapshot) {}
        fn notify(&self, _: &str, _: &str, _: &str) {}
        fn save(&self, _: u64, _: &[crate::office::SavedDesk]) {}
    }

    #[test]
    fn words_for_a_field_go_to_the_window_as_plain_text_and_stop_with_the_recording() {
        let scratch = std::env::temp_dir().join(format!("moshpit-voice-field-{}", std::process::id()));
        let voice = Voice::new(scratch.join("models"), scratch.join("keys.json"), |_| {});
        let engine = crate::engine::start(Arc::new(NoDesktop), vec![], vec![], vec![]);
        let got: Arc<Mutex<Vec<(String, bool)>>> = Arc::default();
        let into = got.clone();
        let open = Arc::new(AtomicBool::new(true));
        let still = open.clone();
        let target = Target { agent: FIELD.into(), run: 7 };
        {
            let mut state = voice.state.lock().unwrap();
            state.target = Some(target.clone());
            state.field = Some(Box::new(move |text: &str, space: bool| {
                into.lock().unwrap().push((text.to_string(), space));
                still.load(Ordering::Acquire)
            }));
            state.view.field = true;
            state.view.phase = Phase::Listening;
        }
        let epoch = voice.state.lock().unwrap().epoch;
        let mut words = Words { voice: &voice, engine: &engine, target: &target, epoch, joiner: Joiner::default(), tentative: String::new() };
        // Each settled piece, with whether a space stands before it; nothing typed into any terminal.
        words.take(Event::Committed { text: "Fix the checkout".into(), boundary: true }, false).unwrap();
        words.take(Event::Committed { text: "total".into(), boundary: true }, false).unwrap();
        words.take(Event::Committed { text: ", then test it.".into(), boundary: true }, false).unwrap();
        // One line of text, whatever the engine said: no new line, and nothing that is not text.
        words.take(Event::Committed { text: "Second\r\nline\u{1b}[31m red\u{3}".into(), boundary: true }, false).unwrap();
        assert_eq!(*got.lock().unwrap(), vec![("Fix the checkout".to_string(), false), ("total".to_string(), true), (", then test it.".to_string(), false), ("Second line red".to_string(), true)]);
        // What is still being decided is shown, not added.
        words.take(Event::Tentative("and one more".into()), false).unwrap();
        assert_eq!((got.lock().unwrap().len(), voice.view().partial.as_str()), (4, "and one more"));

        // The window no longer taking words ends it, and says so.
        open.store(false, Ordering::Release);
        assert!(words.take(Event::Committed { text: "late".into(), boundary: true }, false).unwrap_err().contains("form was closed"));
        // Cancelled, nothing more is added, and it is no longer a recording for a field.
        open.store(true, Ordering::Release);
        voice.cancel();
        let before = got.lock().unwrap().len();
        assert!(words.take(Event::Committed { text: "after cancel".into(), boundary: true }, false).is_err());
        assert_eq!(got.lock().unwrap().len(), before);
        let state = voice.state.lock().unwrap();
        assert!(state.field.is_none() && !state.view.field && state.target.is_none());
        drop(state);

        // A recording for a terminal ends with that desk's program, and with no other's.
        let pane = Target { agent: "desk".into(), run: 1 };
        {
            let mut state = voice.state.lock().unwrap();
            state.mode = Arc::new(AtomicU8::new(0));
            state.target = Some(pane.clone());
        }
        let epoch = voice.state.lock().unwrap().epoch;
        voice.cancel_agent("another desk");
        assert!(voice.state.lock().unwrap().accepts(epoch, &pane));
        voice.cancel_agent("desk");
        assert!(!voice.state.lock().unwrap().accepts(epoch, &pane));
        engine.shutdown();
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn default_is_off_local_and_sends_nothing() {
        let settings = Settings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.engine, Engine::Local);
        assert_eq!(settings.local_model, models::DEFAULT);
        assert_eq!(settings.microphone, None);
        assert!(settings.send_desk_names);
    }

    #[test]
    fn earlier_and_later_settings_are_read_without_losing_the_rest() {
        // 0.6's Whisper settings: the model and language are let go, the rest kept.
        let old: Settings = serde_json::from_str(r#"{"enabled":true,"model":"small","language":"hindi","shortcut":"altspace","microphone":"USB"}"#).unwrap();
        assert!(old.enabled);
        assert_eq!(old.engine, Engine::Local);
        assert_eq!(old.local_model, models::DEFAULT);
        assert_eq!(old.shortcut, Shortcut::Altspace);
        assert_eq!(old.microphone.as_deref(), Some("USB"));
        // A value from a newer version, or a mistake, costs only itself.
        let odd: Settings = serde_json::from_str(r#"{"enabled":true,"engine":"someday","shortcut":7,"vocabulary":"Kubernetes","send_desk_names":"no"}"#).unwrap();
        assert!(odd.enabled);
        assert_eq!(odd.engine, Engine::Local);
        assert_eq!(odd.shortcut, Shortcut::Space);
        assert!(odd.vocabulary.is_empty());
        assert!(odd.send_desk_names);
        let cloud: Settings = serde_json::from_str(r#"{"engine":"deepgram","vocabulary":["Kubernetes"],"send_desk_names":false}"#).unwrap();
        assert_eq!((cloud.engine, cloud.vocabulary.as_slice(), cloud.send_desk_names), (Engine::Deepgram, &["Kubernetes".to_string()][..], false));
    }

    /// 0.6.0 must still read what this version writes, in case someone goes back to it.
    #[test]
    fn the_previous_version_reads_these_settings() {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        #[allow(dead_code)]
        struct Previous {
            enabled: bool,
            model: PreviousModel,
            language: PreviousLanguage,
            shortcut: Shortcut,
            microphone: Option<String>,
        }
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "lowercase")]
        enum PreviousModel {
            Small,
            #[default]
            Base,
        }
        #[derive(Deserialize, Default)]
        #[serde(rename_all = "lowercase")]
        enum PreviousLanguage {
            #[default]
            Auto,
            English,
            Hindi,
        }
        let written = serde_json::to_string(&Settings { enabled: true, engine: Engine::Openai, vocabulary: vec!["Codex".into()], ..Settings::default() }).unwrap();
        let read: Previous = serde_json::from_str(&written).unwrap();
        assert!(read.enabled);
    }

    #[test]
    fn only_a_recording_can_stop_and_cancel_invalidates_every_phase() {
        for phase in [Phase::Idle, Phase::Preparing, Phase::Listening, Phase::Transcribing, Phase::Error] {
            let mut state = Machine::default();
            state.view.phase = phase;
            let target = Target { agent: "pane".into(), run: 3 };
            state.target = Some(target.clone());
            // A shortcut released while the microphone opens stops that recording too.
            if matches!(phase, Phase::Listening | Phase::Preparing) {
                assert!(state.stop().is_ok());
                assert_eq!(state.view.phase, Phase::Transcribing);
                assert_eq!(state.mode.load(Ordering::Acquire), 1);
            } else {
                assert!(state.stop().is_err());
            }
            state.cancel();
            assert_eq!(state.view.phase, Phase::Idle);
            assert!(!state.accepts(0, &target));
            assert!(state.view.agent.is_none());
        }
    }
}
