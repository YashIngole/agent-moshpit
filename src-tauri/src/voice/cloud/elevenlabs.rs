//! ElevenLabs Scribe v2 Realtime (`/v1/speech-to-text/realtime`), English, settled by hand:
//! audio goes as base64 16-bit samples; the phrase being spoken is shown, and at each pause
//! it is committed and typed. ElevenLabs keeps request logs unless the account is Enterprise.
use super::Adapter;
use crate::voice::audio::{pcm16, RATE};
use crate::voice::session::Event;
use base64::Engine as _;
use serde_json::{json, Value};
use tungstenite::Message;

/// 100 ms of audio per message.
const CHUNK: usize = 1_600;
/// Committing often makes Scribe worse; a phrase shorter than this waits for the next pause.
const SHORTEST: usize = 2 * RATE;

pub struct ElevenLabs {
    terms: Vec<String>,
    waiting: Vec<f32>,
    uncommitted: usize,
    /// Commits not yet answered with a committed transcript.
    pending: usize,
    finishing: bool,
}

impl ElevenLabs {
    pub fn new(words: &[String]) -> Self {
        Self { terms: super::terms(words, 50, 20), waiting: Vec::new(), uncommitted: 0, pending: 0, finishing: false }
    }

    fn chunk(pcm: &[f32], commit: bool) -> Message {
        Message::text(json!({ "message_type": "input_audio_chunk", "audio_base_64": base64::engine::general_purpose::STANDARD.encode(pcm16(pcm)), "commit": commit, "sample_rate": RATE }).to_string())
    }

    /// Everything waiting, the last of it marked as the end of the phrase.
    fn commit(&mut self) -> Vec<Message> {
        let rest = std::mem::take(&mut self.waiting);
        let mut out: Vec<Message> = rest.chunks(CHUNK).map(|chunk| Self::chunk(chunk, false)).collect();
        out.push(Self::chunk(&[], true));
        self.uncommitted = 0;
        self.pending += 1;
        out
    }
}

impl Adapter for ElevenLabs {
    fn url(&self) -> String {
        let mut url = url::Url::parse("wss://api.elevenlabs.io/v1/speech-to-text/realtime").unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("model_id", "scribe_v2_realtime");
            query.append_pair("audio_format", "pcm_16000");
            query.append_pair("commit_strategy", "manual");
            query.append_pair("language_code", "en");
            for term in &self.terms {
                query.append_pair("keyterms", term);
            }
        }
        url.into()
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        vec![("xi-api-key", key.to_string())]
    }

    fn audio(&mut self, pcm: &[f32]) -> Vec<Message> {
        self.uncommitted += pcm.len();
        self.waiting.extend_from_slice(pcm);
        let whole = self.waiting.len() / CHUNK * CHUNK;
        let out = self.waiting[..whole].chunks(CHUNK).map(|chunk| Self::chunk(chunk, false)).collect();
        self.waiting.drain(..whole);
        out
    }

    fn pause(&mut self) -> Vec<Message> {
        if self.uncommitted < SHORTEST {
            return Vec::new();
        }
        self.commit()
    }

    fn finish(&mut self) -> Vec<Message> {
        self.finishing = true;
        if self.uncommitted == 0 {
            return Vec::new();
        }
        self.commit()
    }

    fn heard(&mut self, text: &str) -> Vec<Event> {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return Vec::new() };
        let said = message["text"].as_str().unwrap_or_default().to_string();
        let error = message["error"].as_str().unwrap_or_default();
        let provider = super::Provider::Elevenlabs;
        match message["message_type"].as_str().unwrap_or_default() {
            "partial_transcript" | "final_transcript" => vec![Event::Tentative(said)],
            "committed_transcript" => {
                self.pending = self.pending.saturating_sub(1);
                let mut events = vec![Event::Tentative(String::new())];
                if !said.trim().is_empty() {
                    events.insert(0, Event::Committed { text: said, boundary: true });
                }
                events
            }
            "auth_error" => vec![Event::Fault(super::rejected(provider))],
            "quota_exceeded" => vec![Event::Fault(super::no_credit(provider))],
            "rate_limited" | "queue_overflow" | "resource_exhausted" | "commit_throttled" => vec![Event::Fault(super::limited(provider))],
            "unaccepted_terms" => vec![Event::Fault("Accept ElevenLabs' terms for speech to text in your ElevenLabs account first.".into())],
            "insufficient_audio_activity" => vec![Event::Fault("ElevenLabs heard no speech and ended the session. Check the microphone, then try again.".into())],
            "session_time_limit_exceeded" => vec![Event::Fault("ElevenLabs ended the session at its time limit. Words already typed stay.".into())],
            "transcriber_error" | "input_error" | "invalid_request" | "chunk_size_exceeded" | "error" => vec![Event::Fault(format!("ElevenLabs reported a problem: {}", if error.is_empty() { "unknown" } else { error }))],
            _ => Vec::new(),
        }
    }

    fn settled(&self) -> bool {
        self.finishing && self.pending == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(message: &Message) -> Value {
        serde_json::from_str(message.to_text().unwrap()).unwrap()
    }

    #[test]
    fn asks_for_scribe_in_english_settled_by_hand_with_short_terms() {
        let el = ElevenLabs::new(&["Claude".into(), "a-term-longer-than-twenty".into()]);
        let url = el.url();
        for part in ["model_id=scribe_v2_realtime", "audio_format=pcm_16000", "commit_strategy=manual", "language_code=en", "keyterms=Claude"] {
            assert!(url.contains(part), "{part} in {url}");
        }
        assert!(!url.contains("longer"));
        assert_eq!(el.headers("k4")[0], ("xi-api-key", "k4".to_string()));
    }

    #[test]
    fn audio_goes_as_base64_and_a_long_enough_phrase_is_committed_at_a_pause() {
        let mut el = ElevenLabs::new(&[]);
        let out = el.audio(&[0.1; 1_000]);
        assert!(out.is_empty());
        let out = el.audio(&[0.1; 2_400]);
        assert_eq!(out.len(), 2);
        assert_eq!(sent(&out[0])["sample_rate"], 16_000);
        assert_eq!(sent(&out[0])["commit"], false);
        assert!(el.pause().is_empty());
        el.audio(&[0.1; 2 * RATE]);
        let paused = el.pause();
        assert_eq!(sent(paused.last().unwrap())["commit"], true);
        assert!(!el.settled());
        assert_eq!(el.heard(r#"{"message_type":"committed_transcript","text":"Run the tests."}"#), [Event::Committed { text: "Run the tests.".into(), boundary: true }, Event::Tentative(String::new())]);
        // Nothing said since: finishing has nothing to wait for.
        assert!(el.finish().is_empty());
        assert!(el.settled());
    }

    #[test]
    fn partials_are_shown_and_errors_explained() {
        let mut el = ElevenLabs::new(&[]);
        assert_eq!(el.heard(r#"{"message_type":"partial_transcript","text":"Hello, how are"}"#), [Event::Tentative("Hello, how are".into())]);
        assert!(matches!(&el.heard(r#"{"message_type":"auth_error","error":"You must be authenticated to use this endpoint."}"#)[0], Event::Fault(why) if why.contains("did not accept the key")));
        assert!(matches!(&el.heard(r#"{"message_type":"quota_exceeded","error":"x"}"#)[0], Event::Fault(why) if why.contains("out of credit")));
        assert!(el.heard(r#"{"message_type":"warning","warning":"x"}"#).is_empty());
        assert!(el.heard(r#"{"message_type":"session_started","session_id":"s","config":{}}"#).is_empty());
    }
}
