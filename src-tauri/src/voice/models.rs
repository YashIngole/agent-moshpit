//! The local speech models: pinned files, downloaded only when asked for. Only a verified
//! complete file is ever used.
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct Model {
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub url: &'static str,
    pub recommended: bool,
}

/// NVIDIA Nemotron Speech Streaming EN 0.6B (the Parakeet family, cache-aware streaming),
/// converted for transcribe.cpp. Measured on a Ryzen 5 5600H: words land about 1.2 s after
/// they are spoken, the last ones about 0.1 s after stopping, at a sixth of the processor
/// (docs/research/voice-engines-2026-10.md). English only. NVIDIA Open Model License.
pub const MODELS: &[Model] = &[
    Model {
        id: "nemotron-streaming-en-q8",
        label: "Nemotron Speech Streaming",
        file: "nemotron-speech-streaming-en-0.6b-Q8_0.gguf",
        bytes: 729_650_176,
        sha256: "90d8c89714cd31efc88be62a40c6b2bea57e0cc2063af1ffe2c28f1a228ca110",
        url: "https://huggingface.co/handy-computer/nemotron-speech-streaming-en-0.6b-gguf/resolve/9789e0ebf77277911272f0d9a35e1646b5aa6004/nemotron-speech-streaming-en-0.6b-Q8_0.gguf",
        recommended: true,
    },
    Model {
        id: "nemotron-streaming-en-q4",
        label: "Nemotron Speech Streaming, smaller",
        file: "nemotron-speech-streaming-en-0.6b-Q4_K_M.gguf",
        bytes: 475_436_032,
        sha256: "dc959ca31499b114e395c44eb4f0778968f20e5cfb03305a08a39925b2da8e1e",
        url: "https://huggingface.co/handy-computer/nemotron-speech-streaming-en-0.6b-gguf/resolve/9789e0ebf77277911272f0d9a35e1646b5aa6004/nemotron-speech-streaming-en-0.6b-Q4_K_M.gguf",
        recommended: false,
    },
];

/// The model a new install offers first.
pub const DEFAULT: &str = "nemotron-streaming-en-q8";

/// Files earlier versions downloaded (Whisper), removed so they do not sit on the disk unused.
pub const RETIRED: &[&str] = &["ggml-base-q5_1.bin", "ggml-small-q5_1.bin", "ggml-base-q5_1.bin.part", "ggml-small-q5_1.bin.part"];

pub fn find(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

/// What a checked model file looked like: kept beside it (`<file>.verified`), so a file that
/// has not changed is not hashed again (730 MB takes a few seconds).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Stamp {
    sha256: String,
    len: u64,
    modified_ms: u128,
}

fn look(path: &Path, sha256: &str) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    let modified_ms = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis();
    meta.is_file().then(|| Stamp { sha256: sha256.into(), len: meta.len(), modified_ms })
}

fn sidecar(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".verified");
    PathBuf::from(name)
}

/// Whether the model's file is in place and checked, hashing it only when it changed since.
pub fn ready(dir: &Path, model: &Model, cancelled: &AtomicBool) -> Result<(), String> {
    let path = dir.join(model.file);
    let now = look(&path, model.sha256).ok_or("Download this voice model before recording.")?;
    let kept: Option<Stamp> = std::fs::read_to_string(sidecar(&path)).ok().and_then(|text| serde_json::from_str(&text).ok());
    if kept.as_ref() == Some(&now) {
        return Ok(());
    }
    let _ = std::fs::remove_file(sidecar(&path));
    if now.len != model.bytes {
        return Err("The voice model is incomplete. Remove it and download it again.".into());
    }
    let file = std::fs::File::open(&path).map_err(|_| "Download this voice model before recording.".to_string())?;
    validate(file, model.bytes, model.sha256, cancelled)?;
    let _ = std::fs::write(sidecar(&path), serde_json::to_string(&now).unwrap_or_default());
    Ok(())
}

/// Takes a model away, its record first, so a half-removed file is never taken as checked.
pub fn remove(dir: &Path, model: &Model) -> Result<(), String> {
    let path = dir.join(model.file);
    let _ = std::fs::remove_file(sidecar(&path));
    for file in [path, dir.join(format!("{}.part", model.file))] {
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("The model could not be removed: {e}")),
        }
    }
    Ok(())
}

pub fn validate(mut reader: impl Read, bytes: u64, hash: &str, cancelled: &AtomicBool) -> Result<(), String> {
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut chunk = vec![0u8; 256 * 1024];
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        let count = reader.read(&mut chunk).map_err(|e| format!("The model could not be read: {e}"))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > bytes {
            return Err("The model has the wrong size. Remove it and download it again.".into());
        }
        digest.update(&chunk[..count]);
    }
    if total != bytes || format!("{:x}", digest.finalize()) != hash {
        return Err("The model did not pass its size and SHA-256 check. Download it again.".into());
    }
    Ok(())
}

pub struct Verified {
    partial: PathBuf,
}
impl Verified {
    /// Moves the checked download into place and records it as checked.
    pub fn promote(self, destination: &Path, sha256: &str, cancelled: &AtomicBool) -> Result<(), String> {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        let _ = std::fs::remove_file(sidecar(destination));
        std::fs::rename(&self.partial, destination).map_err(|e| format!("The verified model could not be moved into place: {e}"))?;
        if let Some(stamp) = look(destination, sha256) {
            let _ = std::fs::write(sidecar(destination), serde_json::to_string(&stamp).unwrap_or_default());
        }
        Ok(())
    }
}
fn verified(partial: &Path, bytes: u64, hash: &str, cancelled: &AtomicBool) -> Result<Verified, String> {
    let file = std::fs::File::open(partial).map_err(|e| e.to_string())?;
    validate(file, bytes, hash, cancelled)?;
    Ok(Verified { partial: partial.into() })
}

/// Downloads into `partial`, carrying on from what an interrupted download left there. The
/// connection may stall for 30 s at most; the whole download may take as long as it needs.
pub fn download(partial: &Path, model: &Model, cancelled: &AtomicBool, mut progress: impl FnMut(u64)) -> Result<Verified, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let have = std::fs::metadata(partial).map(|m| m.len()).unwrap_or(0).min(model.bytes);
    let mut request = client.get(model.url);
    if have > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let mut response = request
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Hugging Face could not be reached: {e}. Check your connection and try again."))?;
    // A host that ignores the range sends the whole file again: start over.
    let resumed = have > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut total = if resumed { have } else { 0 };
    if response.content_length().is_some_and(|n| n + total != model.bytes) {
        return Err("The model host returned the wrong size. Try again later.".into());
    }
    let mut file = if resumed {
        std::fs::OpenOptions::new().append(true).open(partial)
    } else {
        std::fs::File::create(partial)
    }
    .map_err(|e| format!("The model could not be saved: {e}. Check free disk space."))?;
    let mut chunk = vec![0u8; 256 * 1024];
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        let count = response.read(&mut chunk).map_err(|e| format!("The model download stopped: {e}. Choose Download again to carry on where it stopped."))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > model.bytes {
            return Err("The model host returned too much data. Try again later.".into());
        }
        file.write_all(&chunk[..count]).map_err(|e| format!("The model could not be saved: {e}. Check free disk space."))?;
        progress(total);
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    verified(partial, model.bytes, model.sha256, cancelled).inspect_err(|_| {
        // A complete file that fails its check is not carried on from next time.
        let _ = std::fs::remove_file(partial);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("moshpit-models-{name}-{}-{}", std::process::id(), crate::model::now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_catalog_is_pinned_and_has_its_default() {
        assert!(find(DEFAULT).is_some_and(|m| m.recommended));
        for model in MODELS {
            assert_eq!(model.sha256.len(), 64, "{}", model.id);
            assert!(model.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(model.url.starts_with("https://huggingface.co/") && model.url.contains("/resolve/") && model.url.ends_with(model.file));
            assert_eq!(MODELS.iter().filter(|m| m.file == model.file || m.id == model.id).count(), 1);
            assert!(!RETIRED.contains(&model.file));
        }
    }

    #[test]
    fn size_hash_and_cancel_are_all_required() {
        let hash = format!("{:x}", Sha256::digest(b"fixture"));
        let no = AtomicBool::new(false);
        assert!(validate(&b"fixture"[..], 7, &hash, &no).is_ok());
        assert!(validate(&b"fixtur"[..], 7, &hash, &no).is_err());
        assert!(validate(&b"fixture!"[..], 7, &hash, &no).is_err());
        assert!(validate(&b"fixturf"[..], 7, &hash, &no).is_err());
        assert!(validate(&b"fixture"[..], 7, &hash, &AtomicBool::new(true)).is_err());
    }

    #[test]
    fn a_checked_file_is_trusted_until_it_changes() {
        let dir = scratch("ready");
        let hash = format!("{:x}", Sha256::digest(b"fixture"));
        let model = Model { id: "t", label: "t", file: "t.gguf", bytes: 7, sha256: Box::leak(hash.into_boxed_str()), url: "", recommended: false };
        let no = AtomicBool::new(false);
        assert!(ready(&dir, &model, &no).is_err());
        std::fs::write(dir.join("t.gguf"), b"fixture").unwrap();
        ready(&dir, &model, &no).unwrap();
        assert!(sidecar(&dir.join("t.gguf")).exists());
        // Changed on disk (same size, other bytes): its record no longer matches, so it is hashed and refused.
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(dir.join("t.gguf"), b"fixturf").unwrap();
        assert!(ready(&dir, &model, &no).is_err());
        assert!(!sidecar(&dir.join("t.gguf")).exists());
        remove(&dir, &model).unwrap();
        assert!(!dir.join("t.gguf").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_a_verified_uncancelled_partial_can_replace_the_complete_file() {
        let dir = scratch("promote");
        let partial = dir.join("model.part");
        let final_path = dir.join("model.bin");
        std::fs::write(&final_path, b"old-complete").unwrap();
        std::fs::write(&partial, b"corrupt").unwrap();
        let hash = format!("{:x}", Sha256::digest(b"fixture"));
        let cancelled = AtomicBool::new(false);
        assert!(verified(&partial, 7, &hash, &cancelled).is_err());
        assert_eq!(std::fs::read(&final_path).unwrap(), b"old-complete");
        std::fs::write(&partial, b"fixture").unwrap();
        let checked = verified(&partial, 7, &hash, &cancelled).unwrap();
        cancelled.store(true, Ordering::Release);
        assert!(checked.promote(&final_path, &hash, &cancelled).is_err());
        assert_eq!(std::fs::read(&final_path).unwrap(), b"old-complete");
        cancelled.store(false, Ordering::Release);
        verified(&partial, 7, &hash, &cancelled).unwrap().promote(&final_path, &hash, &cancelled).unwrap();
        assert_eq!(std::fs::read(&final_path).unwrap(), b"fixture");
        assert!(!partial.exists());
        assert!(sidecar(&final_path).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
