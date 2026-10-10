//! Local dictation. A capture owns one terminal run, and never submits a prompt.
mod audio;
mod models;
pub use audio::{inputs, Inputs};
pub use models::Model;

use crate::engine::Handle;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub model: Model,
    pub language: Language,
    pub shortcut: Shortcut,
    pub microphone: Option<String>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Auto,
    English,
    Hindi,
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

#[derive(Debug, Clone, Serialize)]
pub struct ModelView {
    pub id: Model,
    pub bytes: u64,
    pub ready: bool,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct View {
    pub sequence: u64,
    pub phase: Phase,
    pub agent: Option<String>,
    pub started_ms: Option<u64>,
    pub message: String,
    pub models: Vec<ModelView>,
    pub downloading: Option<Model>,
    pub received: u64,
    pub download_error: String,
    pub verifying: bool,
    pub busy: bool,
    pub level: u32,
    pub microphone: String,
    pub transcribing_ms: Option<u64>,
}

#[derive(Default)]
struct Machine {
    view: View,
    epoch: u64,
    target: Option<Target>,
    // 0 captures, 1 stops and transcribes, 2 cancels and discards.
    mode: Arc<AtomicU8>,
    /// Models whose files passed their full check, as they looked then.
    ready: Vec<(Model, models::Stamp)>,
    download_cancel: Option<Arc<AtomicBool>>,
    window_open: bool,
}
impl Machine {
    fn changed(&mut self) {
        self.view.sequence += 1;
    }
    fn has(&self, model: Model) -> bool {
        self.ready.iter().any(|(m, _)| *m == model)
    }
    fn accepts(&self, epoch: u64, target: &Target) -> bool {
        self.epoch == epoch
            && self.target.as_ref() == Some(target)
            && self.mode.load(Ordering::Acquire) != 2
    }
    fn cancel(&mut self) {
        self.mode.store(2, Ordering::Release);
        self.epoch += 1;
        self.target = None;
        self.view.phase = Phase::Idle;
        self.view.agent = None;
        self.view.started_ms = None;
        self.view.message.clear();
        self.view.level = 0;
        self.view.microphone.clear();
        self.view.transcribing_ms = None;
        self.changed();
    }

    fn stop(&mut self) -> Result<(), String> {
        if self.view.phase != Phase::Listening {
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

/// How long a loaded recognizer is kept after its last recording.
const KEEP_LOADED: Duration = Duration::from_secs(5 * 60);

/// The recognizer from the last recording, so the next one need not load it again.
#[derive(Default)]
struct Loaded {
    model: Option<(Model, models::Stamp, Arc<WhisperContext>)>,
    uses: u64,
}

#[derive(Clone)]
pub struct Voice {
    state: Arc<Mutex<Machine>>,
    dir: PathBuf,
    // File validation, removal and promotion cannot race each other. A download
    // writes its own partial file and takes this only to move it into place.
    disk: Arc<Mutex<()>>,
    loaded: Arc<Mutex<Loaded>>,
}

impl Voice {
    pub fn new(dir: PathBuf) -> Self {
        let voice = Self {
            state: Arc::new(Mutex::new(Machine::default())),
            dir,
            disk: Arc::new(Mutex::new(())),
            loaded: Arc::new(Mutex::new(Loaded::default())),
        };
        voice.state.lock().unwrap().view.verifying = true;
        let check = voice.clone();
        std::thread::spawn(move || {
            let _disk = check.disk.lock().unwrap();
            let mut ready = Vec::new();
            for model in [Model::Small, Model::Base] {
                // An interrupted download is never offered as a model.
                let _ = std::fs::remove_file(check.dir.join(format!("{}.part", model.file())));
                let path = check.dir.join(model.file());
                if models::verify(&path, model, &AtomicBool::new(false)).is_ok() {
                    if let Some(stamp) = models::stamp(&path) {
                        ready.push((model, stamp));
                    }
                }
            }
            let mut state = check.state.lock().unwrap();
            state.ready = ready;
            state.view.verifying = false;
            state.changed();
        });
        voice
    }

    pub fn view(&self) -> View {
        let state = self.state.lock().unwrap();
        View {
            models: [Model::Small, Model::Base]
                .map(|id| ModelView {
                    id,
                    bytes: id.bytes(),
                    ready: state.has(id),
                })
                .to_vec(),
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
    pub fn watch(&self, agents: &[String]) {
        let mut state = self.state.lock().unwrap();
        if state
            .target
            .as_ref()
            .is_some_and(|t| !agents.contains(&t.agent))
        {
            state.cancel();
        }
    }
    pub fn shutdown(&self) {
        self.cancel();
        self.cancel_download();
    }
    pub fn stop(&self) -> Result<(), String> {
        self.state.lock().unwrap().stop()
    }

    pub fn start(&self, engine: Handle, agent: String, settings: Settings) -> Result<(), String> {
        if !settings.enabled {
            return Err("Enable local voice in the Voice input panel first.".into());
        }
        let target = engine
            .voice_target(&agent)
            .ok_or("Select a visible terminal whose program is running before recording.")?;
        let mut state = self.state.lock().unwrap();
        if !state.window_open {
            return Err("Open the office window before recording.".into());
        }
        if state.view.busy {
            return Err("The previous voice worker is finishing. Try again in a moment.".into());
        }
        if !state.has(settings.model) {
            return Err("Download the selected voice model first, then try again.".into());
        }
        state.cancel();
        state.mode = Arc::new(AtomicU8::new(0));
        state.target = Some(target.clone());
        state.view.phase = Phase::Preparing;
        state.view.agent = Some(agent);
        state.view.busy = true;
        state.changed();
        let (epoch, mode) = (state.epoch, state.mode.clone());
        let voice = self.clone();
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                voice.record(&engine, &target, epoch, &mode, &settings)
            }));
            voice.release_later();
            let mut state = voice.state.lock().unwrap();
            if state.accepts(epoch, &target) {
                match outcome {
                    Ok(Ok(text)) => {
                        // The same text path as terminal keys. PTY generation is checked under
                        // the write lock; a restart cannot redirect text into its replacement.
                        match engine.voice_write(&target, text.as_bytes()) {
                            Ok(()) => {
                                state.view.phase = Phase::Idle;
                                state.view.message = "Text inserted. Review it in the terminal; press Enter yourself to send.".into();
                            }
                            Err(why) => {
                                state.view.phase = Phase::Error;
                                state.view.message = why;
                            }
                        }
                    }
                    Ok(Err(why)) => {
                        state.view.phase = Phase::Error;
                        state.view.message = why;
                    }
                    Err(_) => {
                        state.view.phase = Phase::Error;
                        state.view.message = "Local voice stopped unexpectedly. Try again.".into();
                    }
                }
                state.target = None;
                state.view.started_ms = None;
                state.view.level = 0;
                state.view.transcribing_ms = None;
            }
            state.view.busy = false;
            state.changed();
        });
        Ok(())
    }

    fn record(
        &self,
        engine: &Handle,
        target: &Target,
        epoch: u64,
        mode: &Arc<AtomicU8>,
        settings: &Settings,
    ) -> Result<String, String> {
        if mode.load(Ordering::Acquire) == 2 || !engine.voice_live(target) {
            return Err("Recording cancelled.".into());
        }
        #[cfg(debug_assertions)]
        let fixture = fixture_pcm()?;
        #[cfg(not(debug_assertions))]
        let fixture: Option<Vec<f32>> = None;
        // An explicit debug fixture NEVER falls back to the microphone, even on failure.
        let capture = if fixture.is_some() {
            None
        } else {
            Some(audio::open(settings.microphone.as_deref())?)
        };
        {
            let mut state = self.state.lock().unwrap();
            if !state.accepts(epoch, target) {
                return Err("Recording cancelled.".into());
            }
            state.view.phase = Phase::Listening;
            state.view.started_ms = Some(crate::model::now_ms());
            state.view.microphone = capture
                .as_ref()
                .map_or_else(|| "Recorded audio fixture".into(), |c| c.name.clone());
            state.changed();
        }
        let began = Instant::now();
        let mut last_count = 0;
        let mut last_audio = Instant::now();
        while mode.load(Ordering::Acquire) == 0
            && !capture.as_ref().is_some_and(|c| {
                c.failed.load(Ordering::Acquire)
                    || c.count.load(Ordering::Acquire) >= audio::MAX_SAMPLES
            })
            && began.elapsed() < Duration::from_secs(audio::MAX_SECONDS as u64)
        {
            if !engine.voice_live(target) {
                self.cancel_agent(&target.agent);
                break;
            }
            if let Some(capture) = &capture {
                let count = capture.count.load(Ordering::Acquire);
                if count != last_count {
                    last_count = count;
                    last_audio = Instant::now();
                }
                if last_audio.elapsed() >= Duration::from_secs(3) {
                    return Err("The microphone is not delivering audio. Check microphone access in system settings, then choose an available input in Voice input.".into());
                }
                let level = capture.level.load(Ordering::Acquire);
                let mut state = self.state.lock().unwrap();
                if state.accepts(epoch, target) && state.view.level != level {
                    state.view.level = level;
                    state.changed();
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Stop capture BEFORE model loading or inference. The stream stays on its owner thread.
        let (mut pcm, failed) = if let Some(audio::DeviceCapture {
            stream,
            audio,
            failed,
            ..
        }) = capture
        {
            drop(stream);
            (
                std::mem::take(&mut audio.lock().unwrap().pcm),
                failed.load(Ordering::Acquire),
            )
        } else {
            (fixture.unwrap_or_default(), false)
        };
        if mode.load(Ordering::Acquire) == 2 {
            return Err("Recording cancelled.".into());
        }
        if failed {
            return Err(
                "The microphone stopped or lost audio. Check the input device and try again."
                    .into(),
            );
        }
        audio::speech(&pcm)?;
        audio::normalize(&mut pcm);
        {
            let mut state = self.state.lock().unwrap();
            if !state.accepts(epoch, target) {
                return Err("Recording cancelled.".into());
            }
            state.view.phase = Phase::Transcribing;
            state.view.level = 0;
            state
                .view
                .transcribing_ms
                .get_or_insert_with(crate::model::now_ms);
            state.changed();
        }
        let path = self.dir.join(settings.model.file());
        let stamp = self.checked(settings.model, &path)?;
        let recognizer = self.recognizer(settings.model, stamp, &path)?;
        transcribe(
            &recognizer,
            &pcm,
            settings.language,
            Abort {
                mode: mode.clone(),
                live: Some((engine.clone(), target.clone())),
            },
        )
    }

    /// The model as it passed its full check, or checked again in full when its file changed since.
    fn checked(&self, model: Model, path: &std::path::Path) -> Result<models::Stamp, String> {
        let _disk = self.disk.lock().unwrap();
        let known = self.state.lock().unwrap().ready.iter().find(|(m, _)| *m == model).map(|(_, s)| *s);
        let now = models::stamp(path).ok_or("Download this voice model before recording.")?;
        if known != Some(now) {
            models::verify(path, model, &AtomicBool::new(false))?;
            let mut state = self.state.lock().unwrap();
            state.ready.retain(|(m, _)| *m != model);
            state.ready.push((model, now));
        }
        Ok(now)
    }

    /// The recognizer for this model file, loaded once and kept between recordings.
    fn recognizer(&self, model: Model, stamp: models::Stamp, path: &std::path::Path) -> Result<Arc<WhisperContext>, String> {
        let mut loaded = self.loaded.lock().unwrap();
        loaded.uses += 1;
        if let Some((m, s, context)) = &loaded.model {
            if *m == model && *s == stamp {
                return Ok(context.clone());
            }
        }
        // Let the previous one go before loading another.
        loaded.model = None;
        let mut context_params = WhisperContextParameters::default();
        context_params.use_gpu(false);
        let context = Arc::new(WhisperContext::new_with_params(path, context_params).map_err(|e| {
            format!("The local model could not load: {e}. Check available memory, or use Base.")
        })?);
        loaded.model = Some((model, stamp, context.clone()));
        Ok(context)
    }

    /// Give back the recognizer's memory once no recording has used it for a while.
    fn release_later(&self) {
        let uses = self.loaded.lock().unwrap().uses;
        let voice = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(KEEP_LOADED);
            let mut loaded = voice.loaded.lock().unwrap();
            if loaded.uses == uses {
                loaded.model = None;
            }
        });
    }

    pub fn download(&self, model: Model) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.view.downloading.is_some() || state.view.verifying {
            return Err("A model is being downloaded or checked. Wait for it to finish.".into());
        }
        if state.has(model) {
            return Ok(());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        state.download_cancel = Some(cancelled.clone());
        state.view.downloading = Some(model);
        state.view.received = 0;
        state.view.download_error.clear();
        state.changed();
        let voice = self.clone();
        std::thread::spawn(move || {
            // Only this download writes its partial file, so recording with a model that is
            // already in place goes on meanwhile. The folder is locked only to move it in.
            let partial = voice.dir.join(format!("{}.part", model.file()));
            let result = (|| {
                std::fs::create_dir_all(&voice.dir)
                    .map_err(|e| format!("The voice model folder could not be made: {e}"))?;
                models::download(&partial, model, &cancelled, |received| {
                    voice.state.lock().unwrap().view.received = received;
                })
            })();
            let _disk = voice.disk.lock().unwrap();
            let mut state = voice.state.lock().unwrap();
            let result = if cancelled.load(Ordering::Acquire) {
                Err("Download cancelled.".into())
            } else {
                result
            };
            let destination = voice.dir.join(model.file());
            let result = result.and_then(|verified| verified.promote(&destination, &cancelled)).and_then(|()| {
                models::stamp(&destination).ok_or_else(|| "The verified model could not be read back.".to_string())
            });
            match result {
                Ok(stamp) => {
                    state.ready.retain(|(m, _)| *m != model);
                    state.ready.push((model, stamp));
                }
                Err(why) => {
                    state.view.download_error = why;
                    let _ = std::fs::remove_file(&partial);
                }
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
    pub fn remove(&self, model: Model) -> Result<(), String> {
        self.cancel();
        let _disk = self.disk.try_lock().map_err(|_| {
            "A model is being checked or downloaded. Cancel it or wait, then try again.".to_string()
        })?;
        let mut state = self.state.lock().unwrap();
        if state.view.busy {
            return Err(
                "The voice worker is finishing. Try removing the model in a moment.".into(),
            );
        }
        let path = self.dir.join(model.file());
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("The model could not be removed: {e}")),
        }
        state.ready.retain(|(m, _)| *m != model);
        let mut loaded = self.loaded.lock().unwrap();
        if loaded.model.as_ref().is_some_and(|(m, _, _)| *m == model) {
            loaded.model = None;
        }
        state.changed();
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
    let mut wav = hound::WavReader::open(path).map_err(|e| {
        format!("The voice WAV fixture could not load: {e}. Microphone was not opened.")
    })?;
    let spec = wav.spec();
    if spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
        || wav.duration() > spec.sample_rate * audio::MAX_SECONDS as u32
    {
        return Err(
            "Use a PCM16 WAV fixture no longer than 60 seconds. Microphone was not opened.".into(),
        );
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

struct Abort {
    mode: Arc<AtomicU8>,
    /// The terminal the text is for. Transcription stops once it is gone. None in tests.
    live: Option<(Handle, Target)>,
}
unsafe extern "C" fn abort(data: *mut std::ffi::c_void) -> bool {
    // `data` points to the stack value in transcribe, alive for the entire full() call.
    let data = unsafe { &*(data as *const Abort) };
    data.mode.load(Ordering::Acquire) == 2 || data.live.as_ref().is_some_and(|(engine, target)| !engine.voice_live(target))
}
fn transcribe(context: &WhisperContext, pcm: &[f32], language: Language, cancellation: Abort) -> Result<String, String> {
    kept(&recognize(context, pcm, language, cancellation)?)
}

/// The text worth inserting from what the model heard: its words, without the notes it
/// writes for sounds that are not speech.
fn kept(heard: &[String]) -> Result<String, String> {
    let text = sanitize(&without_sound_notes(&heard.concat()));
    if text.is_empty() {
        return Err("No words were recognised. Check the language and try again.".into());
    }
    Ok(text)
}

/// What the model heard: each passage, with how likely the model thinks it is no speech at all.
fn recognize(
    context: &WhisperContext,
    pcm: &[f32],
    language: Language,
    mut cancellation: Abort,
) -> Result<Vec<String>, String> {
    let mut state = context
        .create_state()
        .map_err(|e| format!("Local voice could not start: {e}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(
        std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).clamp(1, 4))
            as i32,
    );
    params.set_language(match language {
        Language::Auto => None,
        Language::English => Some("en"),
        Language::Hindi => Some("hi"),
    });
    params.set_translate(false);
    params.set_no_context(true);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    // Use an explicitly scoped callback. The pinned wrapper's "safe" abort helper
    // erases its boxed closure type before casting it back; we do not use that helper.
    unsafe {
        params.set_abort_callback(Some(abort));
        params.set_abort_callback_user_data((&mut cancellation as *mut Abort).cast());
    }
    state
        .full(params, pcm)
        .map_err(|e| format!("Local transcription stopped: {e}. Try a shorter recording."))?;
    let mut heard = Vec::new();
    let mut length = 0;
    for segment in state.as_iter() {
        let text = segment.to_str().map_err(|e| format!("The transcript could not be read: {e}"))?.to_string();
        length += text.len();
        if length > 32 * 1024 {
            return Err("The transcript was too long. Try a shorter recording.".into());
        }
        heard.push(text);
    }
    Ok(heard)
}

/// The model's notes for sounds that are not speech: `[BLANK_AUDIO]` for a fan or a breath,
/// `[APPLAUSE]` for typing, `(music)`, `*sighs*`, `♪ … ♪`. Its no-speech score was no help
/// (about 1e-8 for noise and speech alike), but these notes are how it says there were no words.
fn without_sound_notes(text: &str) -> String {
    let mut words = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(['[', '(', '*', '♪']) {
        let open = rest[start..].chars().next().unwrap();
        let close = match open { '[' => ']', '(' => ')', other => other };
        let after = start + open.len_utf8();
        match rest[after..].find(close) {
            Some(length) => {
                words.push_str(&rest[..start]);
                words.push(' ');
                rest = &rest[after + length + close.len_utf8()..];
            }
            // A lone bracket is kept as it is: only a closed note is dropped.
            None => {
                words.push_str(&rest[..after]);
                rest = &rest[after..];
            }
        }
    }
    words.push_str(rest);
    words
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
                c if c.is_control()
                    || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') => {}
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
            Escape::StringEsc => {
                mode = if c == '\\' {
                    Escape::Plain
                } else {
                    Escape::String
                }
            }
        }
    }
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_sequences_and_enter_never_survive() {
        assert_eq!(
            sanitize(" fix\r\nfile\t\x1b[31mred\x1b[0m\x00\x03\x7f नमस्ते "),
            "fix file red नमस्ते"
        );
        assert_eq!(
            sanitize("a\x1b]52;c;clipboard\x07b\x1bPcommand\x1b\\c"),
            "abc"
        );
        assert_eq!(sanitize("a\u{9b}31mb\u{9d}secret\u{9c}c"), "abc");
        assert_eq!(sanitize("safe\x1b]unterminated"), "safe");
    }
    #[test]
    fn cancel_and_new_generation_reject_stale_results() {
        let mut state = Machine::default();
        let target = Target {
            agent: "desk".into(),
            run: 1,
        };
        state.target = Some(target.clone());
        state.view.phase = Phase::Listening;
        assert!(state.accepts(0, &target));
        assert!(!state.accepts(
            0,
            &Target {
                run: 2,
                ..target.clone()
            }
        ));
        state.cancel();
        assert!(!state.accepts(0, &target));
        state.mode = Arc::new(AtomicU8::new(0));
        state.target = Some(target.clone());
        assert!(!state.accepts(0, &target));
        assert!(state.accepts(1, &target));
    }
    /// What a recording becomes after capture, with the real Base model: the speech check,
    /// amplification, recognition and the no-speech filter, as `record` runs them.
    /// `MOSHPIT_VOICE_FIXTURES` names the folder of the desktop voice test (docs/development.md).
    #[test]
    #[ignore = "needs the Base model and jfk.wav in MOSHPIT_VOICE_FIXTURES"]
    fn the_real_model_hears_short_speech_and_not_noise() {
        let fixtures = PathBuf::from(std::env::var_os("MOSHPIT_VOICE_FIXTURES").expect("MOSHPIT_VOICE_FIXTURES"));
        let mut params = WhisperContextParameters::default();
        params.use_gpu(false);
        let context = WhisperContext::new_with_params(fixtures.join(Model::Base.file()), params).unwrap();
        let wav = hound::WavReader::open(fixtures.join("jfk.wav")).unwrap();
        let jfk: Vec<f32> = wav.into_samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
        let at = |from: f32, to: f32| jfk[(from * 16_000.0) as usize..(to * 16_000.0) as usize].to_vec();
        let hear = |name: &str, clip: Vec<f32>| -> Result<String, String> {
            let result = audio::speech(&clip).and_then(|()| {
                let mut clip = clip;
                audio::normalize(&mut clip);
                let heard = recognize(&context, &clip, Language::English, Abort { mode: Arc::new(AtomicU8::new(0)), live: None })?;
                eprintln!("{name}: the model wrote {heard:?}");
                kept(&heard)
            });
            eprintln!("{name}: {result:?}");
            result
        };
        let mut x = 7u32;
        let mut noise = || { x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (x >> 8) as f32 / (1 << 23) as f32 - 1.0 };
        let n = 32_000;
        let fan: Vec<f32> = (0..n).map(|i| noise() * 0.02 * (1.0 + 0.2 * (std::f32::consts::TAU * 1.5 * i as f32 / 16_000.0).sin())).collect();
        let mut low = 0.0f32;
        let rumble: Vec<f32> = (0..n).map(|_| { low = 0.98 * low + noise() * 0.2; low * 0.03 }).collect();
        let typing: Vec<f32> = (0..n).map(|i| if i % 2_400 < 60 { noise() * 0.3 * (1.0 - (i % 2_400) as f32 / 60.0) } else { noise() * 0.002 }).collect();
        let breath: Vec<f32> = (0..n).map(|i| noise() * 0.01 * (std::f32::consts::PI * i as f32 / n as f32).sin()).collect();
        let white: Vec<f32> = (0..n).map(|_| noise() * 0.02).collect();
        let hum: Vec<f32> = (0..n).map(|i| 0.02 * (std::f32::consts::TAU * 50.0 * i as f32 / 16_000.0).sin()).collect();

        // Speech, including short phrases with no pause around them, is heard as words.
        assert!(hear("whole sample", jfk.clone()).unwrap().contains("ask not what your country can do for you"));
        assert!(hear("1.40-2.00", at(1.4, 2.0)).unwrap().contains("America"));
        assert!(hear("9.60-10.20", at(9.6, 10.2)).unwrap().contains("country"));
        assert!(hear("3.00-3.60", at(3.0, 3.6)).is_ok());
        // Hiss and hum stop at the speech check; the model writes only sound notes for the
        // rest of the noise, so nothing is inserted.
        for (name, clip) in [("white", white), ("hum", hum), ("fan", fan), ("rumble", rumble), ("typing", typing), ("breath", breath)] {
            assert!(hear(name, clip).is_err(), "{name}");
        }
    }

    #[test]
    fn sound_notes_are_not_words() {
        for note in [" [BLANK_AUDIO]", " [APPLAUSE]", "(music)", " *sighs*", "♪ la la ♪", "[ Silence ] (wind)"] {
            assert!(kept(&[note.into()]).is_err(), "{note:?}");
        }
        assert_eq!(kept(&[" Fix the [BLANK_AUDIO] failing test.".into()]).unwrap(), "Fix the failing test.");
        assert_eq!(kept(&[" Hello".into(), " world.".into()]).unwrap(), "Hello world.");
        // A bracket that never closes is not a note; a later note still goes.
        assert_eq!(kept(&["array[0 is empty (music)".into()]).unwrap(), "array[0 is empty");
    }

    #[test]
    fn default_is_off_and_multilingual_without_sending() {
        let settings = Settings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.language, Language::Auto);
        assert_eq!(settings.model, Model::Base);
        assert_eq!(settings.microphone, None);
        let legacy: Settings = serde_json::from_str(r#"{"enabled":true,"model":"small"}"#).unwrap();
        assert_eq!(legacy.model, Model::Small);
        assert_eq!(legacy.microphone, None);
    }
    #[test]
    fn only_listening_can_stop_and_cancel_invalidates_every_phase() {
        for phase in [
            Phase::Idle,
            Phase::Preparing,
            Phase::Listening,
            Phase::Transcribing,
            Phase::Error,
        ] {
            let mut state = Machine::default();
            state.view.phase = phase;
            let target = Target {
                agent: "pane".into(),
                run: 3,
            };
            state.target = Some(target.clone());
            if phase == Phase::Listening {
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
