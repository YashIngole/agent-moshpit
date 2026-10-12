// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The speech worker: this program started again to run the local speech engine apart
    // from the app (see voice/local).
    if std::env::args().any(|arg| arg == "--voice-worker") {
        std::process::exit(agent_moshpit_lib::run_voice_worker());
    }
    // Hooks of Claude Code's, started by it in a session of this office (see mcp/bridge).
    let words: Vec<String> = std::env::args().collect();
    if let Some(at) = words.iter().position(|arg| arg == "--mcp-hook") {
        std::process::exit(agent_moshpit_lib::run_mcp_hook(words.get(at + 1).map_or("", String::as_str)));
    }
    if words.iter().any(|arg| arg == "--mcp-wait") {
        std::process::exit(agent_moshpit_lib::run_mcp_wait());
    }
    if std::env::args().any(|arg| arg == "--mcp") {
        if let Err(error) = agent_moshpit_lib::run_mcp_stdio() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    agent_moshpit_lib::run()
}
