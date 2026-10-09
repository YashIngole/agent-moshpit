//! Saved desks must survive unreadable files and delayed writes during shutdown.
use crate::office::SavedDesk;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct State {
    blocked: bool,
    revision: u64,
    save_failed: bool,
}

pub struct DeskStore {
    pub path: PathBuf,
    state: Mutex<State>,
}

impl DeskStore {
    pub fn load(path: PathBuf) -> (Self, Vec<SavedDesk>, Option<String>) {
        let loaded = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error),
        };
        let (desks, problem) = match loaded {
            Ok(desks) => (desks, None),
            Err(error) => (Vec::new(), Some(format!("Saved desks could not be read: {error}. The original file and terminal screens are preserved. Repair {} and restart the office; new desks cannot be saved until then.", path.display()))),
        };
        let store = Self { path, state: Mutex::new(State { blocked: problem.is_some(), revision: 0, save_failed: false }) };
        (store, desks, problem)
    }

    pub fn blocked(&self) -> bool {
        self.state.lock().unwrap().blocked
    }

    pub fn preserve_screens(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.blocked || state.save_failed
    }

    pub fn save(&self, revision: u64, desks: &[SavedDesk]) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.blocked {
            return Err(io::Error::other("The unreadable saved desks are protected. Repair desks.json and restart before saving changes."));
        }
        if revision <= state.revision {
            return Ok(());
        }
        state.save_failed = true;
        let text = serde_json::to_vec_pretty(desks)?;
        // Keep the last readable version as a separate recovery file.
        match std::fs::read(&self.path) {
            Ok(previous) => {
                // A file modified outside the app is protected too.
                serde_json::from_slice::<Vec<SavedDesk>>(&previous).map_err(io::Error::other)?;
                write_whole(&self.path.with_extension("json.bak"), &previous)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        write_whole(&self.path, &text)?;
        state.revision = revision;
        state.save_failed = false;
        Ok(())
    }
}

pub fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("moshpit-storage-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn desk(title: &str) -> SavedDesk {
        serde_json::from_value(serde_json::json!({"id":"a","harness":"codex","title":title,"cwd":"project"})).unwrap()
    }

    #[test]
    fn corrupt_data_is_reported_and_never_overwritten() {
        let dir = fixture("corrupt");
        let path = dir.join("desks.json");
        std::fs::write(&path, b"[{broken but recoverable").unwrap();
        let (store, loaded, problem) = DeskStore::load(path.clone());
        assert!(loaded.is_empty() && problem.is_some() && store.blocked());
        assert!(store.save(1, &[desk("new")]).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"[{broken but recoverable");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn final_save_wins_over_older_queued_saves_and_keeps_a_backup() {
        let dir = fixture("ordering");
        let path = dir.join("desks.json");
        let (store, _, problem) = DeskStore::load(path.clone());
        assert!(problem.is_none());
        store.save(1, &[desk("before")]).unwrap();
        store.save(3, &[desk("final")]).unwrap();
        store.save(2, &[desk("stale queued message")]).unwrap();
        let (_, loaded, _) = DeskStore::load(path.clone());
        assert_eq!(loaded[0].title, "final");
        let backup: Vec<SavedDesk> = serde_json::from_slice(&std::fs::read(path.with_extension("json.bak")).unwrap()).unwrap();
        assert_eq!(backup[0].title, "before");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_writes_are_errors_not_successful_saves() {
        let dir = fixture("write-error");
        let path = dir.join("missing-parent/desks.json");
        let (store, _, _) = DeskStore::load(path);
        assert!(store.save(1, &[desk("kept in memory")]).is_err());
        std::fs::create_dir(dir.join("missing-parent")).unwrap();
        store.save(1, &[desk("retried")]).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
