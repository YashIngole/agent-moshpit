//! AssemblyAI Universal-Streaming (`/v3/ws`), English. Audio goes as binary 16-bit frames.
//! A turn is shown while it is spoken and typed once it ends, formatted. AssemblyAI bills for
//! as long as the socket is open, so it is always terminated, and opened per recording.
use super::Adapter;
use crate::voice::audio::pcm16;
use crate::voice::session::Event;
use serde_json::Value;
use tungstenite::Message;

/// 100 ms of audio per frame (AssemblyAI takes 50 to 1000 ms).
const CHUNK: usize = 1_600;

pub struct AssemblyAi {
    terms: Vec<String>,
    waiting: Vec<f32>,
    /// The last turn typed, so a turn is never typed twice.
    typed: Option<i64>,
    terminated: bool,
}

impl AssemblyAi {
    pub fn new(words: &[String]) -> Self {
        Self { terms: super::terms(words, 100, 50), waiting: Vec::new(), typed: None, terminated: false }
    }
}

impl Adapter for AssemblyAi {
    fn url(&self) -> String {
        let mut url = url::Url::parse("wss://streaming.assemblyai.com/v3/ws").unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("sample_rate", "16000");
            query.append_pair("encoding", "pcm_s16le");
            query.append_pair("speech_model", "universal-streaming-english");
            query.append_pair("format_turns", "true");
            if !self.terms.is_empty() {
                query.append_pair("keyterms_prompt", &serde_json::to_string(&self.terms).unwrap_or_default());
            }
        }
        url.into()
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        vec![("Authorization", key.to_string())]
    }

    fn audio(&mut self, pcm: &[f32]) -> Vec<Message> {
        self.waiting.extend_from_slice(pcm);
        let whole = self.waiting.len() / CHUNK * CHUNK;
        let frames = self.waiting[..whole].chunks(CHUNK).map(|chunk| Message::binary(pcm16(chunk))).collect();
        self.waiting.drain(..whole);
        frames
    }

    fn finish(&mut self) -> Vec<Message> {
        let mut out = Vec::new();
        if !self.waiting.is_empty() {
            // Under 50 ms is refused: the last scrap is padded with silence.
            let mut rest = std::mem::take(&mut self.waiting);
            rest.resize(rest.len().max(CHUNK / 2), 0.0);
            out.push(Message::binary(pcm16(&rest)));
        }
        // The turn being spoken ends now; Terminate then ends the session, after the last turn.
        out.push(Message::text(r#"{"type":"ForceEndpoint"}"#));
        out.push(Message::text(r#"{"type":"Terminate"}"#));
        self.terminated = true;
        out
    }

    fn heard(&mut self, text: &str) -> Vec<Event> {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return Vec::new() };
        match message["type"].as_str() {
            Some("Turn") => {
                let said = message["transcript"].as_str().unwrap_or_default().to_string();
                let order = message["turn_order"].as_i64().unwrap_or(-1);
                let ended = message["end_of_turn"].as_bool() == Some(true);
                let formatted = message["turn_is_formatted"].as_bool() == Some(true);
                if ended && formatted {
                    if self.typed.is_some_and(|t| t >= order) {
                        return Vec::new();
                    }
                    self.typed = Some(order);
                    let mut events = vec![Event::Tentative(String::new())];
                    if !said.trim().is_empty() {
                        events.insert(0, Event::Committed { text: said, boundary: true });
                    }
                    events
                } else if self.typed.is_some_and(|t| t >= order) {
                    Vec::new()
                } else {
                    // `transcript` holds only the turn's settled words; the newest are in `words`.
                    let words: Vec<&str> = message["words"].as_array().map(|w| w.iter().filter_map(|w| w["text"].as_str()).collect()).unwrap_or_default();
                    vec![Event::Tentative(if words.is_empty() { said } else { words.join(" ") })]
                }
            }
            Some("Termination") => vec![Event::Final],
            Some("Error") => vec![Event::Fault(problem(message["error_code"].as_i64().unwrap_or(0), message["error"].as_str().unwrap_or("unknown")))],
            _ => Vec::new(),
        }
    }

    fn closing(&mut self) -> Vec<Message> {
        if self.terminated {
            Vec::new()
        } else {
            self.terminated = true;
            vec![Message::text(r#"{"type":"Terminate"}"#)]
        }
    }
}

/// An error AssemblyAI sent before closing, by its code and words.
fn problem(code: i64, said: &str) -> String {
    let provider = super::Provider::Assemblyai;
    let lower = said.to_lowercase();
    if lower.contains("too many concurrent sessions") || code == 3009 {
        super::limited(provider)
    } else if lower.contains("invalid api key") || lower.contains("missing authorization") {
        super::rejected(provider)
    } else if code == 1008 {
        let reason = said.trim_start_matches("Unauthorized Connection:").trim();
        format!("AssemblyAI refused this account: {reason}. Check the account's balance and status with AssemblyAI.")
    } else {
        format!("AssemblyAI reported a problem: {said}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_for_english_formatted_turns_with_the_terms() {
        let ai = AssemblyAi::new(&["Claude".into(), "Codex".into()]);
        let url = ai.url();
        for part in ["sample_rate=16000", "encoding=pcm_s16le", "speech_model=universal-streaming-english", "format_turns=true", "keyterms_prompt=%5B%22Claude%22%2C%22Codex%22%5D"] {
            assert!(url.contains(part), "{part} in {url}");
        }
        assert_eq!(ai.headers("k2")[0], ("Authorization", "k2".to_string()));
    }

    #[test]
    fn a_turn_is_shown_then_typed_once_formatted() {
        let mut ai = AssemblyAi::new(&[]);
        let spoken = r#"{"type":"Turn","turn_order":0,"end_of_turn":false,"turn_is_formatted":false,"transcript":"run cargo clippy"}"#;
        assert_eq!(ai.heard(spoken), [Event::Tentative("run cargo clippy".into())]);
        let ended = r#"{"type":"Turn","turn_order":0,"end_of_turn":true,"turn_is_formatted":false,"transcript":"run cargo clippy with all targets"}"#;
        assert_eq!(ai.heard(ended), [Event::Tentative("run cargo clippy with all targets".into())]);
        let formatted = r#"{"type":"Turn","turn_order":0,"end_of_turn":true,"turn_is_formatted":true,"transcript":"Run cargo clippy with all targets."}"#;
        assert_eq!(ai.heard(formatted), [Event::Committed { text: "Run cargo clippy with all targets.".into(), boundary: true }, Event::Tentative(String::new())]);
        // Said again, it is not typed again.
        assert!(ai.heard(formatted).is_empty());
        assert_eq!(ai.heard(r#"{"type":"Termination","audio_duration_seconds":3}"#), [Event::Final]);
    }

    #[test]
    fn the_newest_words_are_shown_and_errors_explained() {
        let mut ai = AssemblyAi::new(&[]);
        let partial = r#"{"turn_order":0,"turn_is_formatted":false,"end_of_turn":false,"transcript":"My name","words":[{"text":"My","word_is_final":true},{"text":"name","word_is_final":true},{"text":"is","word_is_final":false}],"type":"Turn"}"#;
        assert_eq!(ai.heard(partial), [Event::Tentative("My name is".into())]);
        let bad = r#"{"type":"Error","error_code":1008,"error":"Unauthorized Connection: Invalid API key"}"#;
        assert!(matches!(&ai.heard(bad)[0], Event::Fault(why) if why.contains("did not accept the key")));
        assert!(problem(1008, "Unauthorized Connection: Too many concurrent sessions").contains("limiting"));
        assert!(problem(1008, "Unauthorized Connection: Account balance is negative").contains("balance is negative"));
    }

    #[test]
    fn it_is_always_terminated() {
        let mut ai = AssemblyAi::new(&[]);
        assert_eq!(ai.closing(), [Message::text(r#"{"type":"Terminate"}"#)]);
        let mut ai = AssemblyAi::new(&[]);
        ai.audio(&[0.1; 100]);
        let last = ai.finish();
        assert!(matches!(&last[0], Message::Binary(b) if b.len() == CHUNK)); // padded to 50 ms
        assert_eq!(&last[1..], [Message::text(r#"{"type":"ForceEndpoint"}"#), Message::text(r#"{"type":"Terminate"}"#)]);
        assert!(ai.closing().is_empty());
    }
}
