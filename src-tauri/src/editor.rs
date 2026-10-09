//! The editor a file path printed in a terminal is opened in: whichever of the
//! usual ones is on this computer, or the one the user picked.
//!
//! Only ever a file or a folder that exists is handed over, as one word: nothing
//! a program prints is run.

use crate::harness::{self, Found};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// How an editor is told the line to open a file at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Goto {
    /// `-g file:line`, as VS Code and the editors made from it take it.
    Flag,
    /// `file:line`, as Zed takes it.
    Plain,
}

struct Known {
    id: &'static str,
    name: &'static str,
    program: &'static str,
    goto: Goto,
    /// Where its installer puts the command, under the user's own programs folder,
    /// for when that folder is not on the PATH the office was given.
    installed: &'static str,
}

/// The editors looked for, in the order one is picked when the user has not said.
const KNOWN: &[Known] = &[
    Known { id: "cursor", name: "Cursor", program: "cursor", goto: Goto::Flag, installed: "cursor/resources/app/bin" },
    Known { id: "code", name: "VS Code", program: "code", goto: Goto::Flag, installed: "Microsoft VS Code/bin" },
    Known { id: "windsurf", name: "Windsurf", program: "windsurf", goto: Goto::Flag, installed: "Windsurf/bin" },
    Known { id: "antigravity", name: "Antigravity", program: "antigravity", goto: Goto::Flag, installed: "Antigravity/bin" },
    Known { id: "zed", name: "Zed", program: "zed", goto: Goto::Plain, installed: "Zed/bin" },
];

/// An editor on this computer, as the window is told of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorView {
    pub id: String,
    pub name: String,
}

/// An editor found, ready to be started.
pub struct Editor {
    known: &'static Known,
    found: Found,
}

impl Editor {
    pub fn name(&self) -> &'static str {
        self.known.name
    }

    /// Open a file, at a line when there is one, or a folder.
    pub fn open(&self, target: &Path, line: Option<u32>) -> std::io::Result<()> {
        let mut words = self.found.before.clone();
        words.extend(words_for(self.known.goto, &shown(target), line.filter(|_| target.is_file())));
        let mut command = std::process::Command::new(&self.found.program);
        // A `.cmd` launcher is started by the standard library itself, which quotes each
        // word for cmd.exe so that nothing in a path can be read as a command.
        command.args(words).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window flashes up for the editor's launcher.
            command.creation_flags(0x0800_0000);
        }
        command.spawn().map(|_| ())
    }
}

/// A path as an editor should be given it: without Windows' `\\?\` in front.
pub fn shown(path: &Path) -> String {
    path.to_string_lossy().trim_start_matches(r"\\?\").to_string()
}

fn words_for(goto: Goto, target: &str, line: Option<u32>) -> Vec<String> {
    match (goto, line) {
        (Goto::Flag, Some(n)) => vec!["-g".into(), format!("{target}:{n}")],
        (Goto::Plain, Some(n)) => vec![format!("{target}:{n}")],
        (_, None) => vec![target.to_string()],
    }
}

/// Where installers put editors for one user, on Windows.
fn installed_folders(known: &Known) -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let mut all = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        all.push(PathBuf::from(local).join("Programs").join(known.installed));
    }
    if let Some(programs) = std::env::var_os("ProgramFiles") {
        all.push(PathBuf::from(programs).join(known.installed));
    }
    all
}

fn locate(known: &'static Known) -> Option<Editor> {
    harness::find_also(known.program, &installed_folders(known)).map(|found| Editor { known, found })
}

/// The editors on this computer, in the order they are offered.
pub fn found() -> Vec<EditorView> {
    KNOWN.iter().filter(|k| locate(k).is_some()).map(|k| EditorView { id: k.id.into(), name: k.name.into() }).collect()
}

/// The editor asked for when it is here, else the first one that is.
pub fn pick(wanted: Option<&str>) -> Option<Editor> {
    let asked = wanted.and_then(|id| KNOWN.iter().find(|k| k.id == id)).and_then(locate);
    asked.or_else(|| KNOWN.iter().find_map(locate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_editor_is_told_the_line_its_own_way() {
        assert_eq!(words_for(Goto::Flag, r"C:\shop\src\app.ts", Some(42)), ["-g", r"C:\shop\src\app.ts:42"]);
        assert_eq!(words_for(Goto::Plain, "/shop/src/app.ts", Some(7)), ["/shop/src/app.ts:7"]);
        // A folder, or a file with no line, is just itself.
        assert_eq!(words_for(Goto::Flag, r"C:\shop", None), [r"C:\shop"]);
        assert_eq!(words_for(Goto::Plain, "/shop", None), ["/shop"]);
        assert_eq!(shown(Path::new(r"\\?\C:\shop\a.ts")), r"C:\shop\a.ts");
    }

    #[test]
    fn the_usual_editors_are_known_in_a_sensible_order() {
        let ids: Vec<&str> = KNOWN.iter().map(|k| k.id).collect();
        assert_eq!(ids, ["cursor", "code", "windsurf", "antigravity", "zed"]);
        assert!(KNOWN.iter().all(|k| (k.goto == Goto::Plain) == (k.id == "zed")));
        // Something that is not an editor is never picked for one.
        assert!(pick(Some("notepad-of-doom")).is_none_or(|e| KNOWN.iter().any(|k| k.name == e.name())));
    }

    #[test]
    fn a_stand_in_editor_is_found_and_started_with_the_file_at_its_line() {
        let dir = std::env::temp_dir().join(format!("moshpit-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let said = dir.join("said.txt");
        let file = dir.join("a b.ts");
        std::fs::write(&file, "x").unwrap();
        #[cfg(windows)]
        let program = {
            let script = dir.join("cursor.cmd");
            std::fs::write(&script, format!("@echo off\r\necho %* > \"{}\"\r\n", said.display())).unwrap();
            script
        };
        #[cfg(unix)]
        let program = {
            use std::os::unix::fs::PermissionsExt;
            let script = dir.join("cursor");
            std::fs::write(&script, format!("#!/bin/sh\necho \"$@\" > '{}'\n", said.display())).unwrap();
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
            script
        };
        let editor = Editor { known: &KNOWN[0], found: harness::find(&program.to_string_lossy()).unwrap() };
        editor.open(&file, Some(42)).unwrap();
        let mut text = String::new();
        for _ in 0..100 {
            text = std::fs::read_to_string(&said).unwrap_or_default();
            if !text.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(text.contains("-g") && text.contains("a b.ts:42"), "{text:?}");
    }
}
