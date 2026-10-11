# Known limits

- macOS builds require 10.15 or newer (Apple Silicon requires macOS 11 or newer).
- Not used by hand on macOS or Linux. CI builds both and runs the automated checks there, and that is all that is known about them.
- Voice's native microphone permissions, default-device routing and unplug/reconnect behavior have not been checked by hand on macOS or Linux. Windows validation uses demo audio, a prerecorded clip fed at speaking pace, and a pretend cloud provider; release CI starts the speech worker on each system, which does not establish microphone behavior. See the [voice validation record](reviews/voice-validation-2026-10-09.md).
- Voice is English only and capped at five minutes. The local engine is CPU-only (Metal on Apple Silicon); on x86-64 it needs AVX2, FMA, F16C and BMI2 (Intel from 2013, AMD from 2015), and without them the panel points to the cloud. Silence and steady hiss or hum are rejected before anything is typed. The local model wrote nothing for synthetic fan noise; that was checked with recorded speech and synthetic noise, not with a real microphone in a noisy room, and cannot rule out invented words. Cloud providers were checked against their documentation, probes with a wrong key, and a pretend provider on this computer; a real recording with each provider's real service was not checked before 0.7.0, and each may change.
- The install commands need `ssh-keygen` from OpenSSH 8.1 or newer to check signatures: present on macOS 10.15 and later, on Linux desktops with the OpenSSH client, and on Windows 10 (2004 or later) and 11 with the OpenSSH Client feature. Without it they stop and install nothing. The scripts themselves are trusted as served from agentmoshpit.com over HTTPS. Voice accuracy for accents and code vocabulary was measured on a handful of recorded clips (see the [engine decision](research/voice-engines-2026-10.md)), which is not a speech-quality benchmark.
- The local model takes about 1.1 GB of memory (0.85 GB for the smaller one) while loaded, and stays loaded for five minutes after the last recording. The first recording after that loads it again, which takes about a second; nothing said meanwhile is lost.
- Words are typed as the engine settles them, and typed words are never revised: a correction the engine makes later shows only in the words not yet typed. There is no automatic fallback from the cloud to this computer, no reconnecting after a dropped connection, no proxy support for cloud voice, and no text rewriting. The voice shortcut can be changed or disabled when a CLI binds it.
- A desk that receives voice text skips the app's saved-screen file for that app run, to avoid storing its echo. The CLI may still save the conversation itself. A stalled model download gives up after 30 seconds without data and can be resumed; capture and transcription cancellation are independent.
- A desk says that an agent is working, not on what. The programs do not say.
- Status read from a terminal's behaviour is a reading, not a report. A program that asks a question without ringing its terminal looks idle or done. Answering a raised hand while the window is behind other windows can show "done" for a moment.
- Sessions started outside the office are not shown and cannot be brought in.
- A real click on a real Windows notification has not been tried by hand, nor has the taskbar flash been seen. What was checked is that the notification carries the desk's address, and that opening that address opens the desk in the running office.
- Whether Claude Code and Codex take a pasted picture's path as the picture has not been checked with the real programs.
- **Work on a separate copy** has not been run end to end. The office passes the program its own flag; the worktree is the program's doing.
- Installing a real package with npm from inside the app has not been run; a stand-in installer was.
- Claude Code's status has not been watched through real work in a test, because that means giving it a real task.
- Programs known only by name, and Gemini CLI, always start afresh. Codex carries on only after the office verifies the session ID reported by its own terminal; legacy guessed IDs require an explicit `/resume` selection. Antigravity CLI and Hermes carry on their latest conversation, not a particular desk's.
- Antigravity CLI's row was written from Google's documentation and has not been run by the makers: how it takes a task (`--prompt-interactive`) and carries on (`--continue`) may need correcting.
- A desk's last screen is written when the office quits, not while it runs, so after a crash an away desk shows an older screen or none.
- On Windows, a program that can only be started through `cmd.exe` cannot be handed a task containing characters `cmd.exe` would read as commands. The office says so and asks you to type the task in the terminal.
- Updating a program while agents are running it can fail on Windows until they stop.
- The native tray click itself is not automated. Pending New agent delivery and listener-registration races are covered.
- One window, no sounds.
- Installers are not signed with a certificate (the macOS app is signed ad hoc only), so Windows and macOS warn the first time.
- The real Windows app fetched the published v0.3.0 installer and verified its signature on 9 October 2026; changing one byte was rejected. The test explicitly prevented installation. Full installer handoff and replacement remain unverified on every platform. Version 0.2.0 has no updater: get 0.3.0 from the download page.

- The Linux GTK dependency tree has two RustSec informational warnings: GLib iterator unsoundness and unmaintained proc-macro-error. See [the dependency audit](development.md#reliability-checks-added-on-9-october-2026).
