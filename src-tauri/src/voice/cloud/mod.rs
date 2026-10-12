//! Cloud dictation with the user's own key. Each provider is an adapter: what to connect to,
//! how to send audio, and what its messages mean. The adapters do no I/O; one thread per
//! recording drives an adapter over a WebSocket (see `net`). OpenRouter is the exception: it
//! has no stream to connect to, and takes each phrase as an upload of its own (`openrouter`).
pub mod assemblyai;
pub mod deepgram;
pub mod elevenlabs;
pub mod net;
pub mod openai;
pub mod openrouter;

use super::session::{Event, Session};
use serde::Serialize;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
use tungstenite::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Deepgram,
    Assemblyai,
    Openai,
    Elevenlabs,
    Openrouter,
}

pub const PROVIDERS: [Provider; 5] = [Provider::Deepgram, Provider::Openai, Provider::Assemblyai, Provider::Elevenlabs, Provider::Openrouter];
/// Starts the error of a key check that never reached the provider, so the window can offer
/// to keep the key unchecked.
pub const UNREACHED: &str = "unreached:";

impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Provider::Deepgram => "deepgram",
            Provider::Assemblyai => "assemblyai",
            Provider::Openai => "openai",
            Provider::Elevenlabs => "elevenlabs",
            Provider::Openrouter => "openrouter",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Provider::Deepgram => "Deepgram",
            Provider::Assemblyai => "AssemblyAI",
            Provider::Openai => "OpenAI",
            Provider::Elevenlabs => "ElevenLabs",
            Provider::Openrouter => "OpenRouter",
        }
    }
    pub fn from_id(id: &str) -> Option<Provider> {
        PROVIDERS.into_iter().find(|p| p.id() == id)
    }
    /// How it is spoken to over a socket. None for the one that has no stream.
    fn adapter(self, terms: &[String]) -> Option<Box<dyn Adapter>> {
        Some(match self {
            Provider::Deepgram => Box::new(deepgram::Deepgram::new(terms)),
            Provider::Assemblyai => Box::new(assemblyai::AssemblyAi::new(terms)),
            Provider::Openai => Box::new(openai::OpenAi::new(terms)),
            Provider::Elevenlabs => Box::new(elevenlabs::ElevenLabs::new(terms)),
            Provider::Openrouter => return None,
        })
    }
}

/// A recording's way to a provider: a socket its words stream over, or, for the provider
/// that takes whole phrases, an upload for each. `words` are the terms to expect; a
/// provider that cannot be told them is not sent them.
pub fn open(provider: Provider, key: String, words: &[String]) -> Box<dyn Session> {
    match provider.adapter(words) {
        Some(adapter) => Box::new(CloudSession::open(provider, adapter, key)),
        None => Box::new(openrouter::Phrases::open(key)),
    }
}

/// One provider's protocol. Nothing here touches the network.
pub trait Adapter: Send {
    fn url(&self) -> String;
    fn headers(&self, key: &str) -> Vec<(&'static str, String)>;
    /// Sent as soon as the socket is open.
    fn opened(&mut self) -> Vec<Message> {
        Vec::new()
    }
    /// Audio heard since the last call; the adapter sends it in its own chunk size.
    fn audio(&mut self, pcm: &[f32]) -> Vec<Message>;
    /// A phrase ended (for providers that settle words only when told).
    fn pause(&mut self) -> Vec<Message> {
        Vec::new()
    }
    /// No more audio: whatever is left, and the request to settle everything.
    fn finish(&mut self) -> Vec<Message>;
    /// A message from the provider, as events.
    fn heard(&mut self, text: &str) -> Vec<Event>;
    /// Something to send when nothing has been sent for this long, to keep the socket open.
    fn idle(&mut self, _quiet: Duration) -> Option<Message> {
        None
    }
    /// Said before closing, so the provider stops (and stops billing) at once.
    fn closing(&mut self) -> Vec<Message> {
        Vec::new()
    }
    /// After `finish`: whether everything asked for has come, for providers that have no
    /// message of their own to say so.
    fn settled(&self) -> bool {
        false
    }
}

pub fn rejected(provider: Provider) -> String {
    format!("{} did not accept the key. Check it, or add it again, in Voice input.", provider.name())
}
pub fn no_credit(provider: Provider) -> String {
    let name = provider.name();
    format!("{name} says the account is out of credit or over its limit. Add credit with {name}, or switch engine in Voice input.")
}
pub fn limited(provider: Provider) -> String {
    format!("{} is limiting requests right now. Wait a moment and try again.", provider.name())
}

/// Words for the provider to expect (program names, the desk's folder and branch, the user's
/// own list): cleaned, without repeats, cut to a provider's limits.
pub fn terms(words: &[String], most: usize, longest: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    words
        .iter()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric() || " ._+#/'-".contains(*c)).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|w| !w.is_empty() && w.chars().count() <= longest && seen.insert(w.to_lowercase()))
        .take(most)
        .collect()
}

/// A provider's answer, for the person dictating. Never contains the key.
pub fn failure(provider: Provider, failure: net::Failure) -> String {
    let name = provider.name();
    match failure {
        net::Failure::Status(401 | 403, _) => rejected(provider),
        net::Failure::Status(402, _) => no_credit(provider),
        net::Failure::Status(429, _) => limited(provider),
        net::Failure::Status(code, _) if code >= 500 => format!("{name} is having trouble (it answered {code}). Try again, or switch engine in Voice input."),
        net::Failure::Status(code, reason) => {
            let reason = reason.trim();
            if reason.is_empty() {
                format!("{name} refused the connection ({code}).")
            } else {
                format!("{name} refused the connection ({code}): {reason}")
            }
        }
        net::Failure::Certificate(_) => format!("The connection to {name} could not be verified. A proxy or antivirus may be intercepting it."),
        net::Failure::Unreachable(_) => format!("{name} could not be reached. Check your connection, or use the local engine in Voice input."),
    }
}

enum Control {
    Audio(Vec<f32>),
    Pause,
    Finish,
    Cancel,
}

pub struct CloudSession {
    control: Sender<Control>,
    events: Receiver<Event>,
    ended: bool,
}

impl CloudSession {
    /// Starts connecting at once; audio fed meanwhile is sent once the socket is open.
    fn open(provider: Provider, mut adapter: Box<dyn Adapter>, key: String) -> Self {
        #[cfg(debug_assertions)]
        let url = test_endpoint(&adapter.url()).unwrap_or_else(|| adapter.url());
        #[cfg(not(debug_assertions))]
        let url = adapter.url();
        let (control, controls) = channel();
        let (sender, events) = channel();
        std::thread::spawn(move || {
            let socket = match net::connect(&url, &adapter.headers(&key)) {
                Ok(socket) => socket,
                Err(why) => {
                    let _ = sender.send(Event::Fault(failure(provider, why)));
                    return;
                }
            };
            drop(key);
            drive(provider, socket, adapter.as_mut(), &controls, &sender);
        });
        Self { control, events, ended: false }
    }
}

/// In a debug build's named instance, `MOSHPIT_VOICE_ENDPOINT` (say `ws://127.0.0.1:9300`) puts a
/// test server in the provider's place: the same path and query, on another host.
#[cfg(debug_assertions)]
fn test_endpoint(real: &str) -> Option<String> {
    let endpoint = std::env::var("MOSHPIT_VOICE_ENDPOINT").ok().filter(|_| crate::instance_name().is_some())?;
    let real = url::Url::parse(real).ok()?;
    Some(format!("{}{}{}", endpoint.trim_end_matches('/'), real.path(), real.query().map(|q| format!("?{q}")).unwrap_or_default()))
}

/// How long the provider has to settle the last words after the recording stops.
const SETTLE: Duration = Duration::from_secs(3);

fn drive(provider: Provider, mut socket: net::Socket, adapter: &mut dyn Adapter, controls: &Receiver<Control>, events: &Sender<Event>) {
    let lost = format!("Lost the connection to {}. Words already typed stay.", provider.name());
    let send = |socket: &mut net::Socket, messages: Vec<Message>| -> bool { messages.into_iter().all(|m| socket.send(m).is_ok()) };
    let mut sent_at = Instant::now();
    let mut finishing: Option<Instant> = None;
    if !send(&mut socket, adapter.opened()) {
        let _ = events.send(Event::Fault(lost));
        return;
    }
    let _ = events.send(Event::Ready);
    let close = |socket: &mut net::Socket, adapter: &mut dyn Adapter| {
        for message in adapter.closing() {
            let _ = socket.send(message);
        }
        let _ = socket.close(None);
        let _ = socket.flush();
    };
    loop {
        // What the recording asked for since the last turn.
        loop {
            match controls.try_recv() {
                Ok(Control::Audio(pcm)) => {
                    let messages = adapter.audio(&pcm);
                    if !messages.is_empty() {
                        sent_at = Instant::now();
                    }
                    if !send(&mut socket, messages) {
                        let _ = events.send(Event::Fault(lost));
                        return;
                    }
                }
                Ok(Control::Pause) => {
                    if !send(&mut socket, adapter.pause()) {
                        let _ = events.send(Event::Fault(lost));
                        return;
                    }
                }
                Ok(Control::Finish) => {
                    finishing = Some(Instant::now());
                    if !send(&mut socket, adapter.finish()) {
                        let _ = events.send(Event::Fault(lost));
                        return;
                    }
                    sent_at = Instant::now();
                    if adapter.settled() {
                        close(&mut socket, adapter);
                        let _ = events.send(Event::Final);
                        return;
                    }
                }
                Ok(Control::Cancel) | Err(TryRecvError::Disconnected) => {
                    close(&mut socket, adapter);
                    return;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        if let Some(message) = adapter.idle(sent_at.elapsed()) {
            sent_at = Instant::now();
            let _ = socket.send(message);
        }
        // What the provider said (each read waits at most 10 ms).
        match socket.read() {
            Ok(Message::Text(text)) => {
                for event in adapter.heard(text.as_str()) {
                    let done = matches!(event, Event::Final | Event::Fault(_));
                    let _ = events.send(event);
                    if done {
                        close(&mut socket, adapter);
                        return;
                    }
                }
                if finishing.is_some() && adapter.settled() {
                    close(&mut socket, adapter);
                    let _ = events.send(Event::Final);
                    return;
                }
            }
            Ok(Message::Close(frame)) => {
                // Closed by the provider after the last words: that is the end. Before then, a fault.
                let _ = events.send(match (finishing, frame) {
                    (Some(_), _) => Event::Final,
                    (None, Some(frame)) if !frame.reason.is_empty() => Event::Fault(format!("{} ended the connection: {}", provider.name(), frame.reason)),
                    (None, _) => Event::Fault(lost),
                });
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                let _ = events.send(if finishing.is_some() { Event::Final } else { Event::Fault(lost) });
                return;
            }
        }
        if finishing.is_some_and(|at| at.elapsed() > SETTLE) {
            // The recording settles what it has; the provider is told to stop.
            close(&mut socket, adapter);
            let _ = events.send(Event::Final);
            return;
        }
    }
}

impl Session for CloudSession {
    fn feed(&mut self, pcm: &[f32]) {
        if !self.ended && !pcm.is_empty() {
            let _ = self.control.send(Control::Audio(pcm.to_vec()));
        }
    }
    fn pause(&mut self) {
        let _ = self.control.send(Control::Pause);
    }
    fn finish(&mut self) {
        let _ = self.control.send(Control::Finish);
    }
    fn cancel(&mut self) {
        self.ended = true;
        let _ = self.control.send(Control::Cancel);
    }
    fn events(&self) -> &Receiver<Event> {
        &self.events
    }
}

impl Drop for CloudSession {
    fn drop(&mut self) {
        // The socket is closed (and billing stopped) however the recording ended.
        let _ = self.control.send(Control::Cancel);
    }
}

/// Checks a key with the provider's cheapest call, before it is kept.
pub fn check(provider: Provider, key: &str) -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let request = match provider {
        // Each provider's own "is this key good" call; none of them is billed.
        Provider::Deepgram => client.get("https://api.deepgram.com/v1/projects").header("Authorization", format!("Token {key}")),
        Provider::Assemblyai => client.get("https://api.assemblyai.com/v2/transcript?limit=1").header("Authorization", key),
        Provider::Openai => client.get("https://api.openai.com/v1/models").header("Authorization", format!("Bearer {key}")),
        Provider::Elevenlabs => client.get("https://api.elevenlabs.io/v1/models").header("xi-api-key", key),
        Provider::Openrouter => client.get("https://openrouter.ai/api/v1/key").header("Authorization", format!("Bearer {key}")),
    };
    let response = request.send().map_err(|_| format!("{UNREACHED}{} could not be reached to check the key. Check your connection, or save the key without checking.", provider.name()))?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        status if provider == Provider::Openrouter => Err(openrouter::refused(status, "")),
        status => Err(failure(provider, net::Failure::Status(status, String::new()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_are_cleaned_unique_and_within_limits() {
        let words: Vec<String> = ["Claude", "claude", "Codex", "fix/checkout-total", "a <script>", "", "  spaced   out ", "averyveryverylongtermname"].map(String::from).to_vec();
        assert_eq!(terms(&words, 100, 50), ["Claude", "Codex", "fix/checkout-total", "a script", "spaced out", "averyveryverylongtermname"]);
        assert_eq!(terms(&words, 2, 50), ["Claude", "Codex"]);
        assert_eq!(terms(&words, 100, 20), ["Claude", "Codex", "fix/checkout-total", "a script", "spaced out"]);
    }

    #[test]
    fn failures_name_the_provider_and_never_the_key() {
        assert!(failure(Provider::Deepgram, net::Failure::Status(401, "bad key sk-123".into())).contains("did not accept the key"));
        assert!(!failure(Provider::Deepgram, net::Failure::Status(401, "bad key sk-123".into())).contains("sk-123"));
        assert!(failure(Provider::Openai, net::Failure::Status(429, String::new())).contains("limiting"));
        assert!(failure(Provider::Assemblyai, net::Failure::Status(402, String::new())).contains("out of credit"));
        assert!(failure(Provider::Elevenlabs, net::Failure::Unreachable("dns".into())).contains("could not be reached"));
        assert!(failure(Provider::Elevenlabs, net::Failure::Certificate("x".into())).contains("could not be verified"));
    }
}
