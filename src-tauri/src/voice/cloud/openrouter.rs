//! OpenRouter speech-to-text: ElevenLabs Scribe v2 behind an OpenRouter key, billed to the
//! user's OpenRouter credits (`POST /api/v1/audio/transcriptions`). It takes a whole piece
//! of audio and answers with its words; there is no stream. So each phrase is uploaded when
//! the pause after it is heard, and typed when its answer comes, a second or two later.
//! Nothing is shown while a phrase is being said, and what is being said when the recording
//! stops goes up as the last one. Phrases are typed in the order they were said, whichever
//! answer comes first.
use super::{failure, net, Provider};
use crate::voice::audio::{pcm16, speech, RATE};
use crate::voice::session::{Event, Session};
use base64::Engine as _;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

const URL: &str = "https://openrouter.ai/api/v1/audio/transcriptions";
const MODEL: &str = "elevenlabs/scribe-v2";
/// A phrase shorter than this waits for the next pause: too little for a request of its own.
const SHORTEST: usize = RATE;
/// Kept from a quiet stretch that is dropped, so the next word's first sound is not cut.
const LEAD: usize = RATE / 5;
/// How long one phrase may take to come back.
const ANSWER: Duration = Duration::from_secs(20);

enum Turn {
    Audio(Vec<f32>),
    Pause,
    Finish,
    Cancel,
    /// A phrase's words, or why it has none, by its place in the order the phrases were said.
    Answer(usize, Result<String, String>),
}

pub struct Phrases {
    turns: Sender<Turn>,
    events: Receiver<Event>,
    ended: bool,
}

impl Phrases {
    pub fn open(key: String) -> Self {
        #[cfg(debug_assertions)]
        let url = test_endpoint().unwrap_or_else(|| URL.to_string());
        #[cfg(not(debug_assertions))]
        let url = URL.to_string();
        Self::open_at(url, key)
    }

    fn open_at(url: String, key: String) -> Self {
        let (turns, taken) = channel();
        let (sender, events) = channel();
        let answers = turns.clone();
        std::thread::spawn(move || {
            let client = match client(&url) {
                Ok(client) => client,
                Err(why) => {
                    let _ = sender.send(Event::Fault(why));
                    return;
                }
            };
            // There is nothing to connect to before the first phrase: it is hearing at once.
            let _ = sender.send(Event::Ready);
            let mut waiting: Vec<f32> = Vec::new();
            // Phrases sent, phrases typed, and the answers that came before their turn.
            let (mut sent, mut typed) = (0usize, 0usize);
            let mut early: HashMap<usize, String> = HashMap::new();
            let mut finishing = false;
            for turn in taken {
                match turn {
                    Turn::Audio(pcm) => waiting.extend_from_slice(&pcm),
                    Turn::Pause | Turn::Finish => {
                        finishing |= matches!(turn, Turn::Finish);
                        if said(&waiting, finishing) {
                            let (client, url, key, answers, place) = (client.clone(), url.clone(), key.clone(), answers.clone(), sent);
                            let phrase = std::mem::take(&mut waiting);
                            sent += 1;
                            std::thread::spawn(move || {
                                let _ = answers.send(Turn::Answer(place, transcribe(&client, &url, &key, &phrase)));
                            });
                        } else if waiting.len() >= SHORTEST {
                            // Only the room: none of it is sent, and none of it is paid for.
                            waiting.drain(..waiting.len() - LEAD);
                        }
                    }
                    Turn::Cancel => return,
                    Turn::Answer(place, Ok(text)) => {
                        early.insert(place, text);
                    }
                    Turn::Answer(_, Err(why)) => {
                        let _ = sender.send(Event::Fault(why));
                        return;
                    }
                }
                while let Some(text) = early.remove(&typed) {
                    typed += 1;
                    let text = without_sounds(&text);
                    if !text.is_empty() {
                        let _ = sender.send(Event::Committed { text, boundary: true });
                    }
                }
                if finishing && typed == sent {
                    let _ = sender.send(Event::Final);
                    return;
                }
            }
        });
        Self { turns, events, ended: false }
    }
}

impl Session for Phrases {
    fn feed(&mut self, pcm: &[f32]) {
        if !self.ended && !pcm.is_empty() {
            let _ = self.turns.send(Turn::Audio(pcm.to_vec()));
        }
    }
    fn pause(&mut self) {
        let _ = self.turns.send(Turn::Pause);
    }
    fn finish(&mut self) {
        let _ = self.turns.send(Turn::Finish);
    }
    fn cancel(&mut self) {
        self.ended = true;
        let _ = self.turns.send(Turn::Cancel);
    }
    fn events(&self) -> &Receiver<Event> {
        &self.events
    }
    /// The last phrase is still to go up and come back when the recording stops.
    fn settle_within(&self) -> Duration {
        ANSWER + Duration::from_secs(5)
    }
}

impl Drop for Phrases {
    fn drop(&mut self) {
        // Nothing more is typed however the recording ended. A phrase already on its way
        // is answered to nobody.
        let _ = self.turns.send(Turn::Cancel);
    }
}

/// Whether this audio is a phrase to send: long enough, and speech rather than the room.
/// The last of a recording may be shorter than a phrase ended by a pause.
fn said(pcm: &[f32], last: bool) -> bool {
    (last || pcm.len() >= SHORTEST) && speech(pcm).is_ok()
}

/// In a debug build's named instance, `MOSHPIT_VOICE_ENDPOINT` (say `ws://127.0.0.1:9300`) puts
/// a test server on this computer in OpenRouter's place, over plain HTTP.
#[cfg(debug_assertions)]
fn test_endpoint() -> Option<String> {
    let endpoint = std::env::var("MOSHPIT_VOICE_ENDPOINT").ok().filter(|_| crate::instance_name().is_some())?;
    let host = endpoint.strip_prefix("ws://").filter(|host| host.starts_with("127.0.0.1") || host.starts_with("localhost"))?;
    Some(format!("http://{}/api/v1/audio/transcriptions", host.trim_end_matches('/')))
}

fn client(url: &str) -> Result<reqwest::blocking::Client, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::blocking::Client::builder()
        // Plain HTTP only ever reaches the test server above.
        .https_only(url.starts_with("https://"))
        .connect_timeout(Duration::from_secs(8))
        .timeout(ANSWER)
        .build()
        .map_err(|_| "Voice input could not start a connection to OpenRouter. Try again.".to_string())
}

/// One phrase, sent and answered. Never contains the key in what it returns.
fn transcribe(client: &reqwest::blocking::Client, url: &str, key: &str, phrase: &[f32]) -> Result<String, String> {
    let body = json!({
        "model": MODEL,
        "input_audio": { "data": base64::engine::general_purpose::STANDARD.encode(wav(phrase)), "format": "wav" },
        "language": "en"
    });
    let response = client.post(url).header("Authorization", format!("Bearer {key}")).header("Content-Type", "application/json").body(body.to_string()).send().map_err(|e| {
        // Reached, and then silent: not the same as never reached.
        if e.is_timeout() && !e.is_connect() {
            "OpenRouter did not answer in time. Words already typed stay.".to_string()
        } else {
            failure(Provider::Openrouter, net::Failure::Unreachable(String::new()))
        }
    })?;
    let status = response.status().as_u16();
    let text = response.text().unwrap_or_default();
    let answer: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let reason = answer["error"]["message"].as_str().unwrap_or_default().replace(key, "").chars().take(200).collect::<String>();
        return Err(refused(status, &reason));
    }
    answer["text"].as_str().map(str::to_string).ok_or_else(|| "OpenRouter answered in a way that was not understood. Words already typed stay.".to_string())
}

/// What OpenRouter's refusal means, for the person dictating.
pub fn refused(status: u16, reason: &str) -> String {
    match status {
        // A key is refused with 401; 403 is a key that may spend no more.
        403 => "OpenRouter says this key has reached its spending limit. Raise the limit on the key, or switch engine in Voice input.".into(),
        401 | 402 | 429 | 500.. => failure(Provider::Openrouter, net::Failure::Status(status, String::new())),
        _ if reason.trim().is_empty() => format!("OpenRouter refused the audio ({status})."),
        _ => format!("OpenRouter refused the audio ({status}): {}", reason.trim()),
    }
}

/// The audio as a WAV file: sixteen-bit samples, one channel, at the rate it was heard.
fn wav(pcm: &[f32]) -> Vec<u8> {
    let samples = pcm16(pcm);
    let mut file = Vec::with_capacity(44 + samples.len());
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
    file.extend_from_slice(b"WAVEfmt ");
    file.extend_from_slice(&16u32.to_le_bytes());
    // Plain samples, one channel.
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(RATE as u32).to_le_bytes());
    file.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    file.extend_from_slice(&2u16.to_le_bytes());
    file.extend_from_slice(&16u16.to_le_bytes());
    file.extend_from_slice(b"data");
    file.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    file.extend_from_slice(&samples);
    file
}

/// The words without the sounds Scribe writes down between them: `(laughs)`, `[music]`.
/// Only a bracket standing on its own, holding nothing but a few plain words, is taken for
/// one: `list[index]`, `f(x)` and `(2.0)` stay.
fn without_sounds(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(['(', '[']) {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let close = if from.starts_with('(') { ')' } else { ']' };
        let alone = out.is_empty() || out.ends_with(char::is_whitespace);
        let sound = from[1..].find(close).filter(|end| *end > 0 && *end <= 40).map(|end| (&from[1..1 + end], &from[end + 2..])).filter(|(inside, after)| {
            alone && inside.chars().all(|c| c.is_alphabetic() || c == ' ' || c == '_' || c == '-') && (after.is_empty() || after.starts_with(|c: char| c.is_whitespace() || ".,!?".contains(c)))
        });
        match sound {
            Some((_, after)) => rest = after,
            None => {
                out.push_str(&from[..1]);
                rest = &from[1..];
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// Speech-like sound: loud and varying, unlike the room around it.
    fn spoken(seconds: f32) -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize).map(|i| 0.3 * (i as f32 * 0.07).sin() * (0.7 + 0.3 * (i as f32 * 0.0008).sin())).collect()
    }
    fn room(seconds: f32) -> Vec<f32> {
        let mut x = 3u32;
        (0..(seconds * RATE as f32) as usize)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((x >> 8) as f32 / (1 << 23) as f32 - 1.0) * 0.003
            })
            .collect()
    }

    /// What a stand-in for OpenRouter was sent: each request's key, model, format and audio.
    type Asked = Arc<Mutex<Vec<(String, String, String, usize)>>>;

    /// A stand-in for OpenRouter on this computer. It answers a phrase of so many samples with
    /// `answer(samples)`: a status, a body, and how long to take over it.
    fn stand_in(answer: impl Fn(usize) -> (u16, String, u64) + Send + Sync + 'static) -> (String, Asked) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/api/v1/audio/transcriptions", listener.local_addr().unwrap().port());
        let asked: Asked = Arc::default();
        let (seen, answer) = (asked.clone(), Arc::new(answer));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let (seen, answer) = (seen.clone(), answer.clone());
                std::thread::spawn(move || {
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 16 * 1024];
                    // The head, then as much body as it says there is.
                    let (head, body) = loop {
                        let Ok(n) = stream.read(&mut buffer) else { return };
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..n]);
                        let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
                        let head = String::from_utf8_lossy(&request[..end]).to_string();
                        let length = head.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            break (head, request[end + 4..end + 4 + length].to_vec());
                        }
                    };
                    let key = head.lines().find_map(|l| l.to_lowercase().starts_with("authorization:").then(|| l[14..].trim().to_string())).unwrap_or_default();
                    let sent: Value = serde_json::from_slice(&body).unwrap();
                    let audio = base64::engine::general_purpose::STANDARD.decode(sent["input_audio"]["data"].as_str().unwrap()).unwrap();
                    assert_eq!((&audio[..4], &audio[8..16], audio.len() % 2), (&b"RIFF"[..], &b"WAVEfmt "[..], 0));
                    let samples = (audio.len() - 44) / 2;
                    seen.lock().unwrap().push((key, sent["model"].as_str().unwrap().into(), sent["input_audio"]["format"].as_str().unwrap().into(), samples));
                    let (status, text, wait) = answer(samples);
                    std::thread::sleep(Duration::from_millis(wait));
                    let _ = write!(stream, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
                });
            }
        });
        (url, asked)
    }

    fn events(session: &Phrases, until_end: bool) -> Vec<Event> {
        let mut all = Vec::new();
        while let Ok(event) = session.events().recv_timeout(Duration::from_secs(if until_end { 10 } else { 1 })) {
            let end = matches!(event, Event::Final | Event::Fault(_));
            all.push(event);
            if end {
                break;
            }
        }
        all
    }

    #[test]
    fn each_phrase_goes_up_at_its_pause_and_they_are_typed_in_the_order_said() {
        // The first answer is the slow one: the second phrase must still wait its turn.
        let (url, asked) = stand_in(|samples| {
            let (nth, wait) = if samples == RATE * 2 { (1, 500) } else if samples == RATE * 3 / 2 { (2, 0) } else { (3, 0) };
            (200, format!(r#"{{"text":"phrase {nth} (laughs)","usage":{{"seconds":2.0}}}}"#), wait)
        });
        let mut session = Phrases::open_at(url, "sk-or-test".into());
        session.feed(&spoken(2.0));
        session.pause();
        session.feed(&spoken(1.5));
        session.pause();
        // What is still being said when the recording stops is the last phrase, however short.
        session.feed(&spoken(0.6));
        session.finish();
        let committed = |text: &str| Event::Committed { text: text.into(), boundary: true };
        assert_eq!(events(&session, true), [Event::Ready, committed("phrase 1"), committed("phrase 2"), committed("phrase 3"), Event::Final]);
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 3);
        assert!(asked.iter().all(|(key, model, format, _)| key == "Bearer sk-or-test" && model == "elevenlabs/scribe-v2" && format == "wav"), "{asked:?}");
        let mut lengths: Vec<usize> = asked.iter().map(|a| a.3).collect();
        lengths.sort_unstable();
        assert_eq!(lengths, [RATE * 6 / 10, RATE * 3 / 2, RATE * 2]);
    }

    #[test]
    fn the_room_is_never_sent_and_a_word_too_short_waits_for_the_next_pause() {
        let (url, asked) = stand_in(|_| (200, r#"{"text":"said"}"#.into(), 0));
        let mut session = Phrases::open_at(url, "sk-or-test".into());
        session.feed(&room(3.0));
        session.pause();
        session.feed(&spoken(0.6));
        session.pause();
        assert_eq!(events(&session, false), [Event::Ready]);
        assert!(asked.lock().unwrap().is_empty());
        session.feed(&spoken(0.8));
        session.pause();
        session.feed(&room(2.0));
        session.finish();
        assert_eq!(events(&session, true), [Event::Committed { text: "said".into(), boundary: true }, Event::Final]);
        // One request: the quiet before it dropped but for a moment's lead, and none for the quiet after.
        assert_eq!(asked.lock().unwrap().iter().map(|a| a.3).collect::<Vec<_>>(), [LEAD + RATE * 14 / 10]);
    }

    #[test]
    fn a_refusal_ends_the_recording_and_says_why_without_the_key() {
        for (status, body, says) in [
            (401, r#"{"error":{"message":"No auth credentials found sk-or-test"}}"#, "did not accept the key"),
            (402, r#"{"error":{"message":"Insufficient credits"}}"#, "out of credit"),
            (403, r#"{"error":{"message":"Key limit exceeded"}}"#, "spending limit"),
            (429, "{}", "limiting requests"),
            (503, "not json", "having trouble"),
            (400, r#"{"error":{"message":"Model does not take this audio"}}"#, "Model does not take this audio"),
            (200, r#"{"words":[]}"#, "not understood"),
        ] {
            let (url, _) = stand_in(move |_| (status, body.into(), 0));
            let mut session = Phrases::open_at(url, "sk-or-test".into());
            session.feed(&spoken(1.5));
            session.finish();
            let all = events(&session, true);
            let Some(Event::Fault(why)) = all.last() else { panic!("{status}: {all:?}") };
            assert!(why.contains(says) && !why.contains("sk-or-test"), "{status}: {why}");
        }
        // Nobody there at all: a port that was free a moment ago, with nothing listening on it.
        let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut session = Phrases::open_at(format!("http://127.0.0.1:{free}/api/v1/audio/transcriptions"), "sk-or-test".into());
        session.feed(&spoken(1.5));
        session.finish();
        assert!(matches!(events(&session, true).last(), Some(Event::Fault(why)) if why.contains("could not be reached")));
    }

    #[test]
    fn a_cancelled_recording_types_nothing_more() {
        let (url, _) = stand_in(|_| (200, r#"{"text":"too late"}"#.into(), 400));
        let mut session = Phrases::open_at(url, "sk-or-test".into());
        session.feed(&spoken(1.5));
        session.pause();
        session.cancel();
        assert_eq!(events(&session, false), [Event::Ready]);
    }

    #[test]
    fn sounds_written_between_the_words_are_left_out() {
        assert_eq!(without_sounds("(laughs) Fix the test. [music]"), "Fix the test.");
        assert_eq!(without_sounds("Fix the (background noise) test, then (sighs)."), "Fix the test, then .");
        assert_eq!(without_sounds("(silence)"), "");
        // Part of what was said, not a sound beside it.
        assert_eq!(without_sounds("Set list[index] to f(x) and call run()."), "Set list[index] to f(x) and call run().");
        assert_eq!(without_sounds("Bump it to (2.0) and (see #12)"), "Bump it to (2.0) and (see #12)");
        assert_eq!(without_sounds("  two   spaces "), "two spaces");
    }

    #[test]
    fn a_phrase_is_a_whole_sound_file() {
        let file = wav(&[0.0, 0.5, -0.5, 1.0]);
        assert_eq!(file.len(), 44 + 8);
        assert_eq!(&file[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!((u16::from_le_bytes(file[22..24].try_into().unwrap()), u32::from_le_bytes(file[24..28].try_into().unwrap()), u16::from_le_bytes(file[34..36].try_into().unwrap())), (1, 16_000, 16));
        assert_eq!(u32::from_le_bytes(file[40..44].try_into().unwrap()), 8);
        assert_eq!(i16::from_le_bytes(file[46..48].try_into().unwrap()), 16384);
    }
}
