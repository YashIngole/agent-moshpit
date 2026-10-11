//! OpenAI Realtime transcription (`gpt-live-transcribe`, `/v1/realtime?intent=transcription`).
//! Audio goes as base64 16-bit samples at 24 kHz (the only rate it takes). Words stream as
//! deltas while audio arrives; at each pause the audio so far is committed, and its final
//! transcript is typed, in the order the pieces were committed.
use super::Adapter;
use crate::voice::audio::{pcm16, Upsampler, RATE};
use crate::voice::session::Event;
use base64::Engine as _;
use serde_json::{json, Value};
use std::collections::HashMap;
use tungstenite::Message;

/// 100 ms at 24 kHz per message.
const CHUNK: usize = 2_400;
/// Committing less is refused, and a phrase this short is better joined to the next.
const SHORTEST: usize = RATE;

pub struct OpenAi {
    terms: Vec<String>,
    upsampler: Upsampler,
    waiting: Vec<f32>,
    /// Audio sent since the last commit, at 16 kHz.
    uncommitted: usize,
    /// Every item in the order the provider first spoke of it, and its words so far.
    order: Vec<String>,
    deltas: HashMap<String, String>,
    /// Items committed and not yet typed, and those finished (typed in commit order).
    committed: Vec<String>,
    done: HashMap<String, String>,
    /// The session settings were accepted.
    configured: bool,
    /// After finishing: the buffer was cleared, so every commit has been answered for.
    cleared: bool,
    finishing: bool,
}

impl OpenAi {
    pub fn new(words: &[String]) -> Self {
        Self {
            terms: super::terms(words, 100, 60),
            upsampler: Upsampler::default(),
            waiting: Vec::new(),
            uncommitted: 0,
            order: Vec::new(),
            deltas: HashMap::new(),
            committed: Vec::new(),
            done: HashMap::new(),
            configured: false,
            cleared: false,
            finishing: false,
        }
    }

    fn append(&mut self, everything: bool) -> Vec<Message> {
        let whole = if everything { self.waiting.len() } else { self.waiting.len() / CHUNK * CHUNK };
        let out = self.waiting[..whole]
            .chunks(CHUNK)
            .map(|chunk| Message::text(json!({ "type": "input_audio_buffer.append", "audio": base64::engine::general_purpose::STANDARD.encode(pcm16(chunk)) }).to_string()))
            .collect();
        self.waiting.drain(..whole);
        out
    }

    /// Finished items, typed in the order they were committed; the rest shown.
    fn settle(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Some(first) = self.committed.first() {
            let Some(text) = self.done.remove(first) else { break };
            let id = self.committed.remove(0);
            self.order.retain(|i| *i != id);
            self.deltas.remove(&id);
            if !text.trim().is_empty() {
                events.push(Event::Committed { text, boundary: true });
            }
        }
        let shown: Vec<&str> = self.order.iter().filter_map(|id| self.deltas.get(id).map(String::as_str)).filter(|t| !t.trim().is_empty()).collect();
        events.push(Event::Tentative(shown.join(" ")));
        events
    }
}

impl Adapter for OpenAi {
    fn url(&self) -> String {
        "wss://api.openai.com/v1/realtime?intent=transcription".into()
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Bearer {key}"))]
    }

    fn opened(&mut self) -> Vec<Message> {
        let mut transcription = json!({ "model": "gpt-live-transcribe", "languages": ["en"], "delay": "low" });
        if !self.terms.is_empty() {
            transcription["keywords"] = json!(self.terms);
        }
        vec![Message::text(
            json!({
                "type": "session.update",
                "session": { "type": "transcription", "audio": { "input": { "format": { "type": "audio/pcm", "rate": 24000 }, "transcription": transcription, "turn_detection": null } } }
            })
            .to_string(),
        )]
    }

    fn audio(&mut self, pcm: &[f32]) -> Vec<Message> {
        self.uncommitted += pcm.len();
        let more = self.upsampler.push(pcm);
        self.waiting.extend_from_slice(&more);
        self.append(false)
    }

    fn pause(&mut self) -> Vec<Message> {
        if self.uncommitted < SHORTEST {
            return Vec::new();
        }
        self.uncommitted = 0;
        let mut out = self.append(true);
        out.push(Message::text(r#"{"type":"input_audio_buffer.commit"}"#));
        out
    }

    fn finish(&mut self) -> Vec<Message> {
        self.finishing = true;
        let mut out = self.append(true);
        if self.uncommitted >= RATE / 10 {
            out.push(Message::text(r#"{"type":"input_audio_buffer.commit"}"#));
        }
        self.uncommitted = 0;
        // Answered after every commit before it: once it comes, nothing more is on its way
        // but the transcripts of what was committed.
        out.push(Message::text(r#"{"type":"input_audio_buffer.clear"}"#));
        out
    }

    fn heard(&mut self, text: &str) -> Vec<Event> {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return Vec::new() };
        let item = message["item_id"].as_str().unwrap_or_default().to_string();
        match message["type"].as_str() {
            Some("session.updated") => {
                self.configured = true;
                Vec::new()
            }
            Some("conversation.item.input_audio_transcription.delta") => {
                if !self.order.contains(&item) {
                    self.order.push(item.clone());
                }
                self.deltas.entry(item).or_default().push_str(message["delta"].as_str().unwrap_or_default());
                self.settle()
            }
            Some("input_audio_buffer.committed") => {
                if !self.order.contains(&item) {
                    self.order.push(item.clone());
                }
                self.committed.push(item);
                Vec::new()
            }
            Some("conversation.item.input_audio_transcription.completed") => {
                self.done.insert(item, message["transcript"].as_str().unwrap_or_default().to_string());
                self.settle()
            }
            // A piece that could not be transcribed is passed over.
            Some("conversation.item.input_audio_transcription.failed") => {
                let code = message["error"]["code"].as_str().unwrap_or_default();
                if let Some(fault) = fatal(code) {
                    return vec![Event::Fault(fault)];
                }
                self.done.insert(item, String::new());
                self.settle()
            }
            Some("input_audio_buffer.cleared") => {
                self.cleared = true;
                Vec::new()
            }
            Some("error") => {
                let code = message["error"]["code"].as_str().unwrap_or_default();
                let said = message["error"]["message"].as_str().unwrap_or("unknown");
                if let Some(fault) = fatal(code) {
                    vec![Event::Fault(fault)]
                } else if !self.configured {
                    // The session settings were refused: it would not be the session asked for.
                    vec![Event::Fault(format!("OpenAI did not accept the transcription settings: {said}"))]
                } else {
                    // Most errors leave the session going (an empty commit, say).
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    fn settled(&self) -> bool {
        self.finishing && self.cleared && self.committed.is_empty()
    }
}

/// The errors that end a recording, by OpenAI's code.
fn fatal(code: &str) -> Option<String> {
    let provider = super::Provider::Openai;
    match code {
        "invalid_api_key" => Some(super::rejected(provider)),
        "insufficient_quota" | "credit_balance_exhausted" => Some(super::no_credit(provider)),
        code if code.ends_with("_spend_limit_exceeded") || code.ends_with("_usage_limit_exceeded") => Some(super::no_credit(provider)),
        "rate_limit_exceeded" | "slow_down" => Some(super::limited(provider)),
        "model_not_found" => Some("This OpenAI account cannot use gpt-live-transcribe. Check the project's model access, or choose another engine in Voice input.".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(message: &Message) -> Value {
        serde_json::from_str(message.to_text().unwrap()).unwrap()
    }

    #[test]
    fn the_session_is_transcription_only_in_english_at_24khz_without_turn_detection() {
        let mut ai = OpenAi::new(&["Claude".into(), "Codex".into()]);
        assert_eq!(ai.url(), "wss://api.openai.com/v1/realtime?intent=transcription");
        assert_eq!(ai.headers("k3")[0], ("Authorization", "Bearer k3".to_string()));
        let update = sent(&ai.opened()[0]);
        let input = &update["session"]["audio"]["input"];
        assert_eq!(update["session"]["type"], "transcription");
        assert_eq!(input["format"], json!({ "type": "audio/pcm", "rate": 24000 }));
        assert_eq!(input["transcription"]["model"], "gpt-live-transcribe");
        assert_eq!(input["transcription"]["languages"], json!(["en"]));
        assert_eq!(input["transcription"]["keywords"], json!(["Claude", "Codex"]));
        assert!(input["turn_detection"].is_null());
    }

    #[test]
    fn audio_goes_as_base64_at_24khz_and_pauses_commit_it() {
        let mut ai = OpenAi::new(&[]);
        // 250 ms at 16 kHz is 375 ms at 24 kHz: two whole messages of 100 ms, the rest kept.
        let out = ai.audio(&[0.25; 4_000]);
        assert_eq!(out.len(), 2);
        let audio = base64::engine::general_purpose::STANDARD.decode(sent(&out[0])["audio"].as_str().unwrap()).unwrap();
        assert_eq!(audio.len(), CHUNK * 2);
        // Too little since the last commit: a pause commits nothing yet.
        assert!(ai.pause().is_empty());
        ai.audio(&[0.25; RATE]);
        let paused = ai.pause();
        assert_eq!(sent(paused.last().unwrap())["type"], "input_audio_buffer.commit");
        let last = ai.finish();
        assert_eq!(sent(last.last().unwrap())["type"], "input_audio_buffer.clear");
    }

    #[test]
    fn deltas_are_shown_and_transcripts_typed_in_commit_order() {
        let mut ai = OpenAi::new(&[]);
        assert!(ai.heard(r#"{"type":"session.updated","session":{"type":"transcription"}}"#).is_empty());
        assert_eq!(ai.heard(r#"{"type":"conversation.item.input_audio_transcription.delta","item_id":"a","delta":"Refactor the"}"#).last(), Some(&Event::Tentative("Refactor the".into())));
        ai.heard(r#"{"type":"input_audio_buffer.committed","item_id":"a","previous_item_id":null}"#);
        ai.heard(r#"{"type":"conversation.item.input_audio_transcription.delta","item_id":"b","delta":"then add"}"#);
        ai.heard(r#"{"type":"input_audio_buffer.committed","item_id":"b","previous_item_id":"a"}"#);
        // The second finishes first: nothing is typed out of order.
        assert_eq!(ai.heard(r#"{"type":"conversation.item.input_audio_transcription.completed","item_id":"b","transcript":"Then add a test."}"#), [Event::Tentative("Refactor the then add".into())]);
        assert_eq!(
            ai.heard(r#"{"type":"conversation.item.input_audio_transcription.completed","item_id":"a","transcript":"Refactor the checkout."}"#),
            [Event::Committed { text: "Refactor the checkout.".into(), boundary: true }, Event::Committed { text: "Then add a test.".into(), boundary: true }, Event::Tentative(String::new())]
        );
        ai.finish();
        assert!(!ai.settled());
        ai.heard(r#"{"type":"input_audio_buffer.cleared"}"#);
        assert!(ai.settled());
    }

    #[test]
    fn a_rejected_key_or_spent_credit_ends_it_and_other_errors_do_not() {
        let mut ai = OpenAi::new(&[]);
        let bad_key = r#"{"type":"error","error":{"type":"invalid_request_error","code":"invalid_api_key","message":"Incorrect API key provided: sk-proj-***0000.","param":null,"event_id":null}}"#;
        assert!(matches!(&ai.heard(bad_key)[0], Event::Fault(why) if why.contains("did not accept the key") && !why.contains("sk-proj")));
        let mut ai = OpenAi::new(&[]);
        ai.heard(r#"{"type":"session.updated"}"#);
        assert!(ai.heard(r#"{"type":"error","error":{"type":"invalid_request_error","code":"input_audio_buffer_commit_empty","message":"empty"}}"#).is_empty());
        assert!(matches!(&ai.heard(r#"{"type":"error","error":{"code":"insufficient_quota","message":"x"}}"#)[0], Event::Fault(why) if why.contains("out of credit")));
    }
}
