//! A newer version of the office itself.
//!
//! The office asks its own releases on GitHub whether a newer one is out: once
//! soon after it starts, then twice a day. It tells the window, which offers the
//! update in its menu. Nothing is fetched or put in place until the user asks,
//! and what is fetched is checked against the key in `tauri.conf.json` before it
//! is used (that part, and the putting in place, are the updater plugin's).
//!
//! `MOSHPIT_NO_UPDATE_CHECK` turns the asking off, as it does for the agent
//! programs' versions. A build being worked on never asks. `MOSHPIT_UPDATE_URL`
//! names somewhere else to ask, and then it is asked whatever the other two say:
//! that is how the tests show the office a newer version without a network.
//! With `MOSHPIT_UPDATE_ONLY_FETCH` an update is fetched and checked and then left
//! alone, so a release's signed files can be tried against the key without
//! installing anything (`tools/e2e/update-fetch.mjs`).

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

/// How long after starting the first question is asked: the window comes first.
const FIRST_AFTER: Duration = Duration::from_secs(8);
/// How often it is asked again.
const EVERY: Duration = Duration::from_secs(12 * 60 * 60);

/// A newer version, as the window is told of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Newer {
    pub version: String,
}

/// The newer version last heard of, and whether it is being put in place right now.
#[derive(Default)]
pub struct Latest {
    pub newer: Mutex<Option<Newer>>,
    busy: Mutex<bool>,
}

/// Whether to ask at all, and where when it is not the usual place.
fn asking() -> Option<Option<String>> {
    let elsewhere = std::env::var("MOSHPIT_UPDATE_URL").ok().filter(|url| !url.trim().is_empty());
    if elsewhere.is_none() && (std::env::var_os("MOSHPIT_NO_UPDATE_CHECK").is_some() || cfg!(debug_assertions)) {
        return None;
    }
    Some(elsewhere)
}

/// The updater, set to end every program tidily before an installer takes over.
fn updater(app: &AppHandle) -> Option<Updater> {
    let elsewhere = asking()?;
    let mut builder = app.updater_builder();
    if let Some(url) = elsewhere {
        builder = builder.endpoints(vec![url.parse().ok()?]).ok()?;
    }
    // An office asked only to fetch tries whatever the address offers, the version it already is included:
    // a signature is made for one version, so a release's own file can only be tried as itself.
    if std::env::var_os("MOSHPIT_UPDATE_ONLY_FETCH").is_some() {
        builder = builder.version_comparator(|_, _| true);
    }
    let handle = app.clone();
    // On Windows the installer starts and the office is ended on the spot: this runs first.
    builder.on_before_exit(move || crate::put_away(&handle)).build().ok()
}

async fn ask(app: &AppHandle) -> Option<Update> {
    updater(app)?.check().await.ok().flatten()
}

/// Ask now, remember the answer and tell the window.
async fn look(app: &AppHandle) {
    let newer = ask(app).await.map(|update| Newer { version: update.version });
    let latest = app.state::<Latest>();
    let changed = {
        let mut known = latest.newer.lock().unwrap();
        let changed = *known != newer;
        *known = newer.clone();
        changed
    };
    if changed {
        let _ = app.emit("office:newer", newer);
    }
}

/// Keep asking for as long as the office runs.
pub fn watch(app: &AppHandle) {
    if asking().is_none() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_AFTER).await;
        loop {
            look(&app).await;
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// Fetch the newer version, check it, put it in place and start the office again.
/// An error is words for the user: what went wrong, and that nothing was changed.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let latest = app.state::<Latest>();
    {
        let mut busy = latest.busy.lock().unwrap();
        if *busy {
            return Err("The update is already on its way.".into());
        }
        *busy = true;
    }
    let done = put_in_place(app).await;
    *latest.busy.lock().unwrap() = false;
    done
}

async fn put_in_place(app: &AppHandle) -> Result<(), String> {
    let update = ask(app).await.ok_or("No newer version was found just now. Nothing was changed.")?;
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|why| format!("The update could not be fetched ({why}). Nothing was changed."))?;
    if std::env::var_os("MOSHPIT_UPDATE_ONLY_FETCH").is_some() {
        return Err(format!("Version {} was fetched and passed its check ({} bytes). It was not put in place: this office was only asked to fetch it.", update.version, bytes.len()));
    }
    // On Windows this does not come back: the installer takes over (see `updater`).
    update.install(bytes).map_err(|why| format!("The update could not be put in place ({why}). Nothing was changed."))?;
    crate::put_away(app);
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_version_is_told_as_its_number() {
        let told = serde_json::to_string(&Newer { version: "0.3.1".into() }).unwrap();
        assert_eq!(told, r#"{"version":"0.3.1"}"#);
        // Nothing is being put in place until someone asks.
        assert!(!*Latest::default().busy.lock().unwrap());
    }
}
