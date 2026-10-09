//! Pinned optional weights. Only verified complete files are usable.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    #[default]
    Small,
    Base,
}

impl Model {
    pub fn bytes(self) -> u64 {
        match self {
            Self::Small => 190_085_487,
            Self::Base => 59_707_625,
        }
    }
    pub fn file(self) -> &'static str {
        match self {
            Self::Small => "ggml-small-q5_1.bin",
            Self::Base => "ggml-base-q5_1.bin",
        }
    }
    pub fn hash(self) -> &'static str {
        match self {
            Self::Small => "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
            Self::Base => "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
        }
    }
    pub fn url(self) -> String {
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/{}", self.file())
    }
}

pub fn validate(
    mut reader: impl Read,
    bytes: u64,
    hash: &str,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut chunk = [0u8; 64 * 1024];
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        let count = reader
            .read(&mut chunk)
            .map_err(|e| format!("The model could not be read: {e}"))?;
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

pub fn verify(path: &Path, model: Model, cancelled: &AtomicBool) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|_| "Download this voice model before recording.".to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() != model.bytes() {
        return Err("The voice model is incomplete. Remove it and download it again.".into());
    }
    validate(file, model.bytes(), model.hash(), cancelled)
}

pub struct Verified {
    partial: PathBuf,
}
impl Verified {
    pub fn promote(self, destination: &Path, cancelled: &AtomicBool) -> Result<(), String> {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        std::fs::rename(self.partial, destination)
            .map_err(|e| format!("The verified model could not be moved into place: {e}"))
    }
}
fn verified(
    partial: &Path,
    bytes: u64,
    hash: &str,
    cancelled: &AtomicBool,
) -> Result<Verified, String> {
    let file = std::fs::File::open(partial).map_err(|e| e.to_string())?;
    validate(file, bytes, hash, cancelled)?;
    Ok(Verified {
        partial: partial.into(),
    })
}

pub fn download(
    partial: &Path,
    model: Model,
    cancelled: &AtomicBool,
    progress: impl Fn(u64),
) -> Result<Verified, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(model.url())
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| {
            format!("Hugging Face could not be reached: {e}. Check your connection and try again.")
        })?;
    if response
        .content_length()
        .is_some_and(|n| n != model.bytes())
    {
        return Err("The model host returned the wrong size. Try again later.".into());
    }
    let mut file = std::fs::File::create(partial)
        .map_err(|e| format!("The model could not be saved: {e}. Check free disk space."))?;
    let mut total = 0u64;
    let mut chunk = [0u8; 64 * 1024];
    let began = std::time::Instant::now();
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        if began.elapsed() > Duration::from_secs(15 * 60) {
            return Err(
                "The model download took too long. Try again on a faster connection.".into(),
            );
        }
        let count = response
            .read(&mut chunk)
            .map_err(|e| format!("The model download stopped: {e}. Try again."))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > model.bytes() {
            return Err("The model host returned too much data. Try again later.".into());
        }
        file.write_all(&chunk[..count])
            .map_err(|e| format!("The model could not be saved: {e}. Check free disk space."))?;
        progress(total);
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    verified(partial, model.bytes(), model.hash(), cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn only_a_verified_uncancelled_partial_can_replace_the_complete_file() {
        let dir = std::env::temp_dir().join(format!(
            "moshpit-download-{}-{}",
            std::process::id(),
            crate::model::now_ms()
        ));
        std::fs::create_dir_all(&dir).unwrap();
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
        assert!(checked.promote(&final_path, &cancelled).is_err());
        assert_eq!(std::fs::read(&final_path).unwrap(), b"old-complete");
        cancelled.store(false, Ordering::Release);
        verified(&partial, 7, &hash, &cancelled)
            .unwrap()
            .promote(&final_path, &cancelled)
            .unwrap();
        assert_eq!(std::fs::read(&final_path).unwrap(), b"fixture");
        assert!(!partial.exists());
        std::fs::remove_file(&final_path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
