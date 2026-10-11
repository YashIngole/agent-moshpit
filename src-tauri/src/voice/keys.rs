//! The user's own keys for cloud dictation. A key is kept in the system's keychain (Windows
//! Credential Manager, the macOS Keychain, the Secret Service on Linux), never in a file and
//! never sent to the window: the window only learns that one is there and its last four
//! characters. Where no keychain can be used, a key can be kept for this session only.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

/// What the window may know about a key: never the key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kept {
    pub last4: String,
    pub saved_ms: u64,
    /// Held in memory only, until the app quits.
    #[serde(skip)]
    pub session_only: bool,
}

/// Starts a message the window recognises: the keychain failed, and a key can still be kept
/// for this session only if the user chooses that.
pub const NO_KEYCHAIN: &str = "keychain:";

pub struct Keys {
    /// Which providers have a key in the keychain, so the app never opens the keychain (and,
    /// on some systems, a prompt) just to draw the Voice panel.
    path: PathBuf,
    kept: Mutex<BTreeMap<String, Kept>>,
    session: Mutex<HashMap<String, String>>,
}

fn service() -> String {
    match crate::instance_name() {
        Some(name) => format!("Agent Moshpit voice ({name})"),
        None => "Agent Moshpit voice".into(),
    }
}

fn entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(&service(), provider).map_err(|e| format!("{NO_KEYCHAIN}The system keychain could not be opened: {e}"))
}

pub fn last4(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    chars[chars.len().saturating_sub(4)..].iter().collect()
}

impl Keys {
    pub fn new(path: PathBuf) -> Self {
        let kept = std::fs::read_to_string(&path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        Self { path, kept: Mutex::new(kept), session: Mutex::default() }
    }

    pub fn list(&self) -> BTreeMap<String, Kept> {
        let mut list = self.kept.lock().unwrap().clone();
        for (provider, key) in self.session.lock().unwrap().iter() {
            list.insert(provider.clone(), Kept { last4: last4(key), saved_ms: crate::model::now_ms(), session_only: true });
        }
        list
    }

    pub fn has(&self, provider: &str) -> bool {
        self.session.lock().unwrap().contains_key(provider) || self.kept.lock().unwrap().contains_key(provider)
    }

    /// The key itself, for a recording or a test. Reads the keychain: never under another lock.
    pub fn get(&self, provider: &str) -> Result<String, String> {
        if let Some(key) = self.session.lock().unwrap().get(provider) {
            return Ok(key.clone());
        }
        if !self.kept.lock().unwrap().contains_key(provider) {
            return Err("Add a key for this provider in Voice input first.".into());
        }
        entry(provider)?.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => "The key for this provider is no longer in the system keychain. Add it again in Voice input.".to_string(),
            other => format!("The key could not be read from the system keychain: {other}"),
        })
    }

    pub fn set(&self, provider: &str, key: &str, session_only: bool) -> Result<(), String> {
        if session_only {
            self.session.lock().unwrap().insert(provider.into(), key.into());
            return Ok(());
        }
        entry(provider)?.set_password(key).map_err(|e| format!("{NO_KEYCHAIN}The system keychain did not take the key: {e}"))?;
        self.session.lock().unwrap().remove(provider);
        let mut kept = self.kept.lock().unwrap();
        kept.insert(provider.into(), Kept { last4: last4(key), saved_ms: crate::model::now_ms(), session_only: false });
        self.save(&kept)
    }

    pub fn remove(&self, provider: &str) -> Result<(), String> {
        self.session.lock().unwrap().remove(provider);
        let mut kept = self.kept.lock().unwrap();
        if kept.remove(provider).is_some() {
            match entry(provider).and_then(|e| e.delete_credential().map_err(|e| e.to_string())) {
                Ok(()) => {}
                Err(why) if why.contains("No matching entry") => {}
                Err(why) => return Err(format!("The key could not be removed from the system keychain: {why}")),
            }
        }
        self.save(&kept)
    }

    fn save(&self, kept: &BTreeMap<String, Kept>) -> Result<(), String> {
        let text = serde_json::to_string_pretty(kept).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &self.path)).map_err(|e| format!("The list of saved keys could not be written: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_learns_only_the_last_four_characters() {
        assert_eq!(last4("sk-abcdef123456"), "3456");
        assert_eq!(last4("ab"), "ab");
        assert_eq!(last4("ключ-ыз"), "ч-ыз");
    }

    #[test]
    fn a_session_key_is_never_written_down() {
        let path = std::env::temp_dir().join(format!("moshpit-keys-{}-{}.json", std::process::id(), crate::model::now_ms()));
        let keys = Keys::new(path.clone());
        keys.set("deepgram", "dg-secret-1234", true).unwrap();
        assert!(keys.has("deepgram"));
        assert_eq!(keys.get("deepgram").unwrap(), "dg-secret-1234");
        assert!(keys.list()["deepgram"].session_only);
        assert_eq!(keys.list()["deepgram"].last4, "1234");
        assert!(!path.exists());
        keys.remove("deepgram").unwrap();
        assert!(!keys.has("deepgram"));
        assert!(keys.get("deepgram").is_err());
        let _ = std::fs::remove_file(&path);
    }

    /// The real keychain of the computer running the tests. Opt-in: it writes and deletes an entry.
    #[test]
    #[ignore = "uses the system keychain"]
    fn the_system_keychain_keeps_a_key() {
        let path = std::env::temp_dir().join(format!("moshpit-keys-real-{}.json", std::process::id()));
        let keys = Keys::new(path.clone());
        keys.set("test-provider", "test-secret-9876", false).unwrap();
        assert_eq!(Keys::new(path.clone()).get("test-provider").unwrap(), "test-secret-9876");
        keys.remove("test-provider").unwrap();
        assert!(Keys::new(path.clone()).get("test-provider").is_err());
        let _ = std::fs::remove_file(&path);
    }
}
