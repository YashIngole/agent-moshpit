//! Deepgram Nova-3 streaming (`/v1/listen`). Audio goes as binary 16-bit frames; interim
//! results are shown, final ones typed. Opted out of Deepgram's model-improvement program.
use super::Adapter;
use crate::voice::audio::pcm16;
use crate::voice::session::Event;
use serde_json::Value;
use std::time::Duration;
use tungstenite::Message;

/// 100 ms of audio per frame.
const CHUNK: usize = 1_600;

pub struct Deepgram {
    terms: Vec<String>,
    waiting: Vec<f32>,
    finishing: bool,
}

impl Deepgram {
    pub fn new(words: &[String]) -> Self {
        // At most 100 terms and about 500 tokens in all; a word is counted as two tokens to stay clear of it.
        let mut tokens = 0;
        let terms = super::terms(words, 100, 50)
            .into_iter()
            .take_while(|t| {
                tokens += t.split_whitespace().count() * 2;
                tokens <= 450
            })
            .collect();
        Self { terms, waiting: Vec::new(), finishing: false }
    }
}

impl Adapter for Deepgram {
    fn url(&self) -> String {
        let mut url = url::Url::parse("wss://api.deepgram.com/v1/listen").unwrap();
        {
            let mut query = url.query_pairs_mut();
            for (name, value) in [
                ("model", "nova-3"),
                ("language", "en"),
                ("encoding", "linear16"),
                ("sample_rate", "16000"),
                ("channels", "1"),
                ("interim_results", "true"),
                ("punctuate", "true"),
                ("smart_format", "true"),
                ("endpointing", "300"),
                ("mip_opt_out", "true"),
            ] {
                query.append_pair(name, value);
            }
            for term in &self.terms {
                query.append_pair("keyterm", term);
            }
        }
        url.into()
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        vec![("Authorization", format!("Token {key}"))]
    }

    fn audio(&mut self, pcm: &[f32]) -> Vec<Message> {
        self.waiting.extend_from_slice(pcm);
        let whole = self.waiting.len() / CHUNK * CHUNK;
        let frames = self.waiting[..whole].chunks(CHUNK).map(|chunk| Message::binary(pcm16(chunk))).collect();
        self.waiting.drain(..whole);
        frames
    }

    fn finish(&mut self) -> Vec<Message> {
        self.finishing = true;
        let mut out = Vec::new();
        if !self.waiting.is_empty() {
            out.push(Message::binary(pcm16(&std::mem::take(&mut self.waiting))));
        }
        // Finalize settles what is buffered at once; CloseStream then ends the stream, after
        // which Deepgram sends what is left and its metadata, and closes.
        out.push(Message::text(r#"{"type":"Finalize"}"#));
        out.push(Message::text(r#"{"type":"CloseStream"}"#));
        out
    }

    fn heard(&mut self, text: &str) -> Vec<Event> {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return Vec::new() };
        match message["type"].as_str() {
            Some("Results") => {
                let said = message["channel"]["alternatives"][0]["transcript"].as_str().unwrap_or_default().to_string();
                if message["is_final"].as_bool() == Some(true) {
                    let mut events = vec![Event::Tentative(String::new())];
                    if !said.trim().is_empty() {
                        events.insert(0, Event::Committed { text: said, boundary: true });
                    }
                    if message["from_finalize"].as_bool() == Some(true) {
                        events.push(Event::Final);
                    }
                    events
                } else {
                    vec![Event::Tentative(said)]
                }
            }
            // Sent once the stream is closed: everything has come.
            Some("Metadata") if self.finishing => vec![Event::Final],
            // Deepgram's error messages leave the stream going; a fatal one closes it instead.
            _ => Vec::new(),
        }
    }

    fn idle(&mut self, quiet: Duration) -> Option<Message> {
        // Deepgram closes a stream that has heard nothing for ten seconds.
        (quiet >= Duration::from_secs(4)).then(|| Message::text(r#"{"type":"KeepAlive"}"#))
    }

    fn closing(&mut self) -> Vec<Message> {
        if self.finishing {
            Vec::new()
        } else {
            vec![Message::text(r#"{"type":"CloseStream"}"#)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_for_english_live_results_and_no_training() {
        let url = Deepgram::new(&["Claude".into(), "fix/checkout-total".into()]).url();
        for part in ["model=nova-3", "language=en", "encoding=linear16", "sample_rate=16000", "interim_results=true", "smart_format=true", "mip_opt_out=true", "keyterm=Claude", "keyterm=fix%2Fcheckout-total"] {
            assert!(url.contains(part), "{part} in {url}");
        }
        assert_eq!(Deepgram::new(&[]).headers("k1")[0], ("Authorization", "Token k1".to_string()));
    }

    #[test]
    fn audio_goes_in_tenths_of_a_second_and_the_rest_on_finishing() {
        let mut dg = Deepgram::new(&[]);
        assert!(dg.audio(&[0.0; 1000]).is_empty());
        let frames = dg.audio(&[0.0; 2500]);
        assert_eq!(frames.len(), 2);
        assert!(frames.iter().all(|f| matches!(f, Message::Binary(b) if b.len() == CHUNK * 2)));
        let last = dg.finish();
        assert!(matches!(&last[0], Message::Binary(b) if b.len() == 300 * 2));
        assert_eq!(last[1], Message::text(r#"{"type":"Finalize"}"#));
    }

    #[test]
    fn interim_words_are_shown_and_final_ones_typed() {
        let mut dg = Deepgram::new(&[]);
        let interim = r#"{"type":"Results","is_final":false,"speech_final":false,"channel":{"alternatives":[{"transcript":"refactor the","confidence":0.9}]}}"#;
        assert_eq!(dg.heard(interim), [Event::Tentative("refactor the".into())]);
        let settled = r#"{"type":"Results","is_final":true,"speech_final":true,"channel":{"alternatives":[{"transcript":"Refactor the checkout.","confidence":0.97}]}}"#;
        assert_eq!(dg.heard(settled), [Event::Committed { text: "Refactor the checkout.".into(), boundary: true }, Event::Tentative(String::new())]);
        dg.finish();
        let last = r#"{"type":"Results","is_final":true,"from_finalize":true,"channel":{"alternatives":[{"transcript":"Then add a test.","confidence":0.95}]}}"#;
        assert_eq!(dg.heard(last), [Event::Committed { text: "Then add a test.".into(), boundary: true }, Event::Tentative(String::new()), Event::Final]);
        assert_eq!(dg.heard(r#"{"type":"Metadata","request_id":"x"}"#), [Event::Final]);
        assert!(dg.heard("not json").is_empty());
    }
}
