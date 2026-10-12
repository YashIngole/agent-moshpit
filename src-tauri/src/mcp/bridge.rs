//! This program started again by a CLI, as one of three small things, each over before
//! the desktop, the window or the one-office check are set up:
//!
//! - `--mcp`: the CLI's MCP server, over its standard streams. It passes each call on to
//!   the office and holds a watch open to it. Everything it writes, answers and events
//!   alike, goes through one writer, so two never run together on the line.
//! - `--mcp-hook <Event>`: a hook of Claude Code's. It asks the office what is new for the
//!   session and says it in the words that hook's event takes.
//! - `--mcp-wait`: a hook that waits. It returns when there is something to wake an idle
//!   session for, and says it in the way that wakes one.
//!
//! A hook that cannot reach the office says nothing and succeeds: nothing here may ever
//! get in the way of the session it runs in.
use super::{launch, protocol, MAX_REQUEST, MAX_RESPONSE};
use serde_json::{json, Value};
use std::io::{self, BufReader, Read};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::time::Duration;

/// Frames waiting to be written to the CLI. An answer waits for room; an event that
/// finds none is not written, and is asked for again.
const OUTBOX: usize = 256;

fn office() -> Result<(SocketAddr, String), String> {
    let address: SocketAddr = std::env::var(launch::ENDPOINT).map_err(|_| "Start this MCP connection from an Agent Moshpit session.".to_string())?.parse().map_err(|_| "Invalid Moshpit MCP endpoint.".to_string())?;
    if !address.ip().is_loopback() { return Err("The Moshpit MCP endpoint must be local.".into()); }
    let token = std::env::var(launch::TOKEN).map_err(|_| "Missing Moshpit session capability.".to_string())?;
    Ok((address, token))
}

/// One thing asked of the office and answered: a tool's call, or a hook's question.
fn ask(address: &SocketAddr, request: &Value, patience: Duration) -> Result<Value, String> {
    let stream = TcpStream::connect_timeout(address, Duration::from_secs(3)).map_err(|_| "The Moshpit office is unavailable. Start or resume your session from Moshpit.".to_string())?;
    stream.set_read_timeout(Some(patience)).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    protocol::write_frame(reader.get_mut(), request).map_err(|e| e.to_string())?;
    let reply = protocol::read_frame(&mut reader, MAX_RESPONSE).map_err(|e| e.to_string())?.ok_or("The office closed the connection.")?;
    let reply: Value = serde_json::from_slice(&reply).map_err(|e| e.to_string())?;
    match reply["error"].as_str() { Some(error) => Err(error.into()), None => reply.get("value").cloned().ok_or_else(|| "Invalid office response.".into()) }
}

/// Entry point used by the CLI's MCP subprocess, before Tauri or single-instance setup.
pub fn run_stdio() -> Result<(), String> {
    let (address, token) = office()?;
    // Only a session started with pushes passes them on, and says that it can.
    let channel = std::env::var(launch::PUSH).is_ok_and(|way| way == "channel");
    let (outbox, frames) = sync_channel::<Value>(OUTBOX);
    // The one writer. When the CLI stops reading, it ends, and so does everything else.
    std::thread::spawn(move || {
        let mut output = io::stdout().lock();
        for frame in frames {
            if protocol::write_frame(&mut output, &frame).is_err() { break; }
        }
    });
    let mut session = protocol::Session { channel, ..Default::default() };
    let mut watching = false;
    let mut input = io::stdin().lock();
    while let Some(frame) = protocol::read_frame(&mut input, MAX_REQUEST).map_err(|e| e.to_string())? {
        let response = match serde_json::from_slice::<Value>(&frame) {
            Ok(request) => {
                let method = request["method"].as_str().unwrap_or("").to_string();
                let response = protocol::respond(request, &mut session, |name, arguments| {
                    ask(&address, &json!({"token":token,"name":name,"arguments":arguments,"cwd":std::env::current_dir().map_err(|e| e.to_string())?}), Duration::from_secs(30))
                });
                // Once the CLI has asked what tools there are (or used one), the watch is
                // opened, and stays for as long as this runs. Not before: an open watch tells
                // the office this session can read what it is told of, and until it has the
                // tools it cannot.
                if !watching && session.initialized && matches!(method.as_str(), "tools/list" | "tools/call") {
                    watching = true;
                    let (outbox, token, client, protocol) = (outbox.clone(), token.clone(), session.client.clone(), session.protocol.clone());
                    std::thread::spawn(move || watch(address, token, outbox, channel, client, protocol));
                }
                response
            }
            Err(_) => Some(protocol::error(Value::Null, -32700, "Invalid JSON")),
        };
        if let Some(response) = response {
            if outbox.send(response).is_err() { break; }
        }
    }
    Ok(())
}

/// The watch: open for as long as this subprocess runs, opened again when it drops. What
/// the office writes down it is passed to the CLI in the form the CLI takes pushes in. An
/// event that could not be passed on is asked for again: `after` only moves past what was.
fn watch(address: SocketAddr, token: String, outbox: SyncSender<Value>, channel: bool, client: String, protocol: String) {
    let (mut after, mut pause) = (0u64, Duration::from_millis(500));
    loop {
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
            // The office says a word at least every twenty seconds. Longer than this, it is gone.
            let _ = stream.set_read_timeout(Some(Duration::from_secs(45)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
            let mut reader = BufReader::new(stream);
            let asked = json!({"token":token,"watch":{"after":after,"push":if channel { "channel" } else { "none" },"client":client,"protocol":protocol}});
            if protocol::write_frame(reader.get_mut(), &asked).is_ok() {
                while let Ok(Some(frame)) = protocol::read_frame(&mut reader, MAX_RESPONSE) {
                    let Ok(frame) = serde_json::from_slice::<Value>(&frame) else { break };
                    // No longer this session's to watch: its program is being stopped.
                    if frame.get("error").is_some() { return; }
                    pause = Duration::from_millis(500);
                    let Some(event) = frame.get("event") else { continue };
                    if channel {
                        match outbox.try_send(json!({"jsonrpc":"2.0","method":"notifications/claude/channel","params":event["channel"]})) {
                            Ok(()) => {}
                            Err(TrySendError::Full(_)) => break,
                            Err(TrySendError::Disconnected(_)) => return,
                        }
                    }
                    after = after.max(event["revision"].as_u64().unwrap_or(0));
                }
            }
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_secs(8));
    }
}

/// Whatever the CLI writes to a hook is read and let go: what the office has to say does
/// not depend on it, and a hook that leaves it unread can hold the CLI up.
fn drain() {
    std::thread::spawn(|| {
        let _ = io::copy(&mut io::stdin().lock().take(64 * 1024 * 1024), &mut io::sink());
    });
}

/// A hook of Claude Code's: what is new for this session, in the words its event takes.
/// The exit code is always success, and nothing is printed when there is nothing new.
pub fn run_hook(event: &str) -> i32 {
    drain();
    if !matches!(event, "PostToolBatch" | "UserPromptSubmit" | "Stop") { return 0; }
    let Ok((address, token)) = office() else { return 0 };
    let Ok(answer) = ask(&address, &json!({"token":token,"hook":{"event":event}}), Duration::from_secs(3)) else { return 0 };
    if let Some(context) = answer["context"].as_str() {
        println!("{}", json!({"hookSpecificOutput":{"hookEventName":event,"additionalContext":context}}));
    }
    0
}

/// A hook that waits: it returns when there is something to wake an idle session for,
/// and says it on the stream, and with the exit code, that wake one. Let go of by the
/// office, or unable to reach it, it ends quietly and wakes nobody.
pub fn run_wait() -> i32 {
    drain();
    let Ok((address, token)) = office() else { return 0 };
    // The CLI ends a hook at its own time limit; this one has none of its own.
    let Ok(answer) = ask(&address, &json!({"token":token,"wait":{}}), Duration::from_secs(24 * 60 * 60)) else { return 0 };
    match answer["context"].as_str() {
        Some(context) => {
            eprintln!("{context}");
            2
        }
        None => 0,
    }
}
