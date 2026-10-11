//! What the app and its speech worker say to each other over the worker's stdin and stdout.
//! Each message is a frame: a kind byte, a little-endian length, and that many bytes. Kind 0
//! is JSON; kind 1 is audio, 32-bit floats at 16 kHz.
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Raised whenever either side's messages change, so an app updated while its worker kept
/// running (or the other way round) is noticed instead of misread.
pub const PROTOCOL: u32 = 1;
/// The largest frame either side accepts: a second of audio is 64 KB.
const MOST: usize = 1 << 20;
const JSON: u8 = 0;
const AUDIO: u8 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToWorker {
    /// Load this model, or keep it if it is the one loaded.
    Load { path: String, threads: i32, right_context: i32 },
    /// A recording begins; audio frames follow. Its id is on everything said about it.
    Begin { id: u64 },
    /// No more audio: settle everything and say Final.
    Finish,
    /// Drop the recording now.
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromWorker {
    Hello { protocol: u32, version: String },
    Ready { load_ms: u64 },
    /// New settled text since the last message, and the text still being decided after it.
    Text { id: u64, committed: String, tentative: String },
    Final { id: u64, committed: String },
    /// `id` is the recording it ended, or 0 for a problem loading the model.
    Error { id: u64, message: String },
}

#[derive(Debug, PartialEq)]
pub enum Frame<T> {
    Message(T),
    Audio(Vec<f32>),
}

pub fn send<T: Serialize>(out: &mut impl Write, message: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(message).map_err(std::io::Error::other)?;
    frame(out, JSON, &body)
}

pub fn send_audio(out: &mut impl Write, pcm: &[f32]) -> std::io::Result<()> {
    let body: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
    frame(out, AUDIO, &body)
}

fn frame(out: &mut impl Write, kind: u8, body: &[u8]) -> std::io::Result<()> {
    if body.len() > MOST {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut head = [0u8; 5];
    head[0] = kind;
    head[1..].copy_from_slice(&(body.len() as u32).to_le_bytes());
    out.write_all(&head)?;
    out.write_all(body)?;
    out.flush()
}

/// The next frame, or None at the end of the stream. Anything malformed is an error, and the
/// reader stops: the other side is not trusted to recover.
pub fn receive<T: for<'de> Deserialize<'de>>(input: &mut impl Read) -> std::io::Result<Option<Frame<T>>> {
    let mut head = [0u8; 5];
    match input.read_exact(&mut head) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if len > MOST {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut body = vec![0u8; len];
    input.read_exact(&mut body)?;
    match head[0] {
        JSON => serde_json::from_slice(&body).map(|m| Some(Frame::Message(m))).map_err(std::io::Error::other),
        AUDIO if len.is_multiple_of(4) => Ok(Some(Frame::Audio(body.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect()))),
        _ => Err(std::io::Error::other("unknown frame")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let mut wire = Vec::new();
        send(&mut wire, &ToWorker::Load { path: "m.gguf".into(), threads: 4, right_context: 6 }).unwrap();
        send_audio(&mut wire, &[0.5, -0.25]).unwrap();
        send(&mut wire, &ToWorker::Finish).unwrap();
        let mut input = &wire[..];
        assert_eq!(receive::<ToWorker>(&mut input).unwrap(), Some(Frame::Message(ToWorker::Load { path: "m.gguf".into(), threads: 4, right_context: 6 })));
        assert_eq!(receive::<ToWorker>(&mut input).unwrap(), Some(Frame::Audio(vec![0.5, -0.25])));
        assert_eq!(receive::<ToWorker>(&mut input).unwrap(), Some(Frame::Message(ToWorker::Finish)));
        assert_eq!(receive::<ToWorker>(&mut input).unwrap(), None);
    }

    #[test]
    fn malformed_frames_are_errors_not_guesses() {
        for wire in [
            vec![7, 0, 0, 0, 0],                         // unknown kind
            vec![0, 2, 0, 0, 0, b'{', b'x'],             // bad JSON
            vec![1, 3, 0, 0, 0, 1, 2, 3],                // audio not whole floats
            vec![0, 0xff, 0xff, 0xff, 0x7f],             // far too large
            vec![0, 5, 0, 0, 0, b'{'],                   // cut short
        ] {
            assert!(receive::<ToWorker>(&mut &wire[..]).is_err(), "{wire:?}");
        }
    }
}
