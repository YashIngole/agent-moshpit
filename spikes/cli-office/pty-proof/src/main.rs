//! Can one windowed app run Claude Code and Codex as its own children, each in a
//! pseudo-terminal, without any of them getting a window of its own?
//!
//!   pty-proof <out dir> <cwd> <seconds to hold> <command line> [<command line> ...]
//!
//! Built for the Windows subsystem, as the real app is, so it has no console to lend.
//! It starts every command in its own pseudo-terminal, writes the process ids to
//! `pids.txt`, holds for a while so `check.ps1` can look at the windows and the process
//! tree, then saves what each program drew to `<n>.bin` and ends them.
#![windows_subsystem = "windows"]

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type Writer = Arc<Mutex<Box<dyn Write + Send>>>;

struct Running {
    child: Box<dyn Child + Send + Sync>,
    /// Kept so the pseudo-terminal stays open for as long as the program runs.
    _master: Box<dyn MasterPty + Send>,
    writer: Writer,
    seen: Arc<Mutex<Vec<u8>>>,
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn start(line: &str, cwd: &PathBuf) -> Result<Running, String> {
    let pair = native_pty_system()
        .openpty(PtySize { rows: 32, cols: 110, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;
    let mut words = line.split(' ');
    let mut command = CommandBuilder::new(words.next().unwrap_or_default());
    command.args(words);
    command.cwd(cwd);
    let child = pair.slave.spawn_command(command).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer: Writer = Arc::new(Mutex::new(pair.master.take_writer().map_err(|e| e.to_string())?));
    let seen = Arc::new(Mutex::new(Vec::new()));

    let (sink, reply) = (seen.clone(), writer.clone());
    thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let bytes = &chunk[..n];
            sink.lock().unwrap().extend_from_slice(bytes);
            // A terminal answers when asked where the cursor is and what it is. Nothing
            // is drawing here, so answer the way a plain one would.
            let mut out = reply.lock().unwrap();
            if contains(bytes, b"\x1b[6n") {
                let _ = out.write_all(b"\x1b[1;1R");
            }
            if contains(bytes, b"\x1b[c") || contains(bytes, b"\x1b[0c") {
                let _ = out.write_all(b"\x1b[?1;2c");
            }
            let _ = out.flush();
        }
    });

    Ok(Running { child, _master: pair.master, writer, seen })
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    let cwd = PathBuf::from(args.next().expect("cwd"));
    let hold: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);

    let mut report = format!("self {}\n", std::process::id());
    let mut running = Vec::new();
    for line in args {
        match start(&line, &cwd) {
            Ok(one) => {
                report.push_str(&format!("child {} {line}\n", one.child.process_id().unwrap_or(0)));
                running.push(one);
            }
            Err(error) => report.push_str(&format!("failed {line}: {error}\n")),
        }
    }
    std::fs::write(out.join("pids.txt"), &report).expect("write pids");

    thread::sleep(Duration::from_secs(hold));

    for (i, one) in running.iter_mut().enumerate() {
        std::fs::write(out.join(format!("{i}.bin")), &*one.seen.lock().unwrap()).expect("write screen");
        // Ask politely first, so each program can tidy up after itself.
        for _ in 0..2 {
            let _ = one.writer.lock().unwrap().write_all(b"\x03");
            thread::sleep(Duration::from_millis(300));
        }
    }
    thread::sleep(Duration::from_millis(1500));
    for one in running.iter_mut() {
        let _ = one.child.kill();
    }
    std::fs::write(out.join("done.txt"), "done").expect("write done");
}
