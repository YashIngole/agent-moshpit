// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The speech worker: this program started again to run the local speech engine apart
    // from the app (see voice/local).
    if std::env::args().any(|arg| arg == "--voice-worker") {
        std::process::exit(agent_moshpit_lib::run_voice_worker());
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
