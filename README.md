# Agent Moshpit

[![CI](https://github.com/YashIngole/agent-moshpit/actions/workflows/ci.yml/badge.svg)](https://github.com/YashIngole/agent-moshpit/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/YashIngole/agent-moshpit)](https://github.com/YashIngole/agent-moshpit/releases/latest)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)

Run multiple Claude Code, Codex and other coding agents side by side in one desktop window. Each is the real command-line program in its own terminal, drawn as a person at a desk. One look tells you who is working, who is done, and which agent is waiting for you.

![The Agent Moshpit window: agents at their desks on the left, three of their terminals in a grid beside them, and an amber band across the top naming the agent who needs you](docs/terminals.png)

**[Download](https://agentmoshpit.com)** · [Guide](docs/guide.md) · [Parallel-agent guides](https://agentmoshpit.com/guides/) · [Release notes](https://github.com/YashIngole/agent-moshpit/releases)

## Features

- **Real terminals.** Each agent is the `claude`, `codex` or `agy` you already have, with your own sign-in and settings. The app draws none of the conversation and keeps no API key.
- **Who needs you.** When an agent stops on a question, a hand goes up at its desk, an amber band names it, the tray icon changes and you get a notification.
- **Side by side.** Open terminals in a grid beside the floor. Resize, rearrange and zoom them. [Every shortcut](docs/guide.md#keys) is listed in the app.
- **Stays out of the way.** Close the window and the agents keep working from the tray. After a restart every desk is back with its last screen, and Claude Code, Codex, Antigravity CLI and Hermes carry on their conversations.
- **Fourteen CLIs, and yours.** Claude Code, Codex, Antigravity CLI, Hermes, Gemini CLI, OpenCode, Copilot CLI, Amp, Qwen Code, Crush, Cursor CLI, Aider, Goose and Droid. Add another with [a few lines of JSON](docs/programs.md#adding-a-program-of-your-own).
- **Updates when you say.** When a newer version is out, the ⋯ menu offers it. One click fetches it, checks it against the app's own key and starts the office again.
- **Private.** No account, no telemetry. See [what it sends](#what-it-sends).
- **Local voice input.** Optional dictation into a selected live terminal. Download a model under **⋯ → Voice input**, then use the mic. Review the text and press Enter yourself; voice never sends it for you.

## Install

**macOS**, in Terminal:

```sh
curl -fsSL https://agentmoshpit.com/install.sh | sh
```

**Windows**: the [installer](https://github.com/YashIngole/agent-moshpit/releases/latest/download/agent-moshpit_windows_x64-setup.exe) (on "Windows protected your PC", choose **More info**, then **Run anyway**), or in PowerShell, which asks nothing:

```powershell
irm https://agentmoshpit.com/install.ps1 | iex
```

**Linux**: the [AppImage](https://github.com/YashIngole/agent-moshpit/releases/latest/download/agent-moshpit_linux_amd64.AppImage), [.deb](https://github.com/YashIngole/agent-moshpit/releases/latest/download/agent-moshpit_linux_amd64.deb) or [.rpm](https://github.com/YashIngole/agent-moshpit/releases/latest/download/agent-moshpit_linux_x86_64.rpm), or the same command as macOS, which picks the right one.

The commands fetch the newest release from this repository, install it and open it; the scripts are [install.sh](site/install.sh) and [install.ps1](site/install.ps1). The app is not code-signed yet, which is why macOS gets a command: a `.dmg` is on the [releases page](https://github.com/YashIngole/agent-moshpit/releases/latest), but macOS refuses to open it until you allow it under **System Settings → Privacy & Security**. Once installed, the app offers its own updates.

You also need at least one agent CLI, installed and signed in the way its makers describe. The app finds the ones you have and can install those published on npm.

## Quick start

1. Press **+ new agent** (or `n`), pick a program and a folder, and give it a task if you like. For Claude and Codex, choose a model, effort and permissions, or inherit your CLI settings. The model picker reads the CLI's catalog; **Refresh** finds new releases.
2. The agent takes a desk and its terminal opens beside the floor. Type to it as you would in any terminal.
3. Hold `Ctrl` and click another desk to open it beside the first. `Ctrl` + `` ` `` goes back to the floor.
4. When a hand goes up, press **open their terminal** on the amber band and answer the question.

The [guide](docs/guide.md) covers the rest: the floor, terminals, the desk menu, notifications and every key.

## Platform support

| System | Status |
| --- | --- |
| Windows 11 | Used every day. Tested by hand and by a test that drives the real app. |
| macOS | 10.15+ on Intel, 11+ on Apple Silicon. Built and checked by CI. Not yet used by hand. |
| Linux | Built and checked by CI. Not yet used by hand. |

On macOS and Linux a few things are missing for now: a click on a notification does not open that agent's desk, the right-click **Paste** may ask before reading the clipboard, and the webview's own keys (reload, for one) are not turned off. If you run either build, please [report what breaks](https://github.com/YashIngole/agent-moshpit/issues). All known gaps are in [limits.md](docs/limits.md).

## What it sends

Nothing about you or your work. It asks two questions about versions, when it starts and every 12 hours: npm, for the newest version of each agent CLI you have installed, and GitHub, for the newest version of itself. Set `MOSHPIT_NO_UPDATE_CHECK` to turn both off. An update is only fetched when you choose it. The programs it runs talk to their own services as they always do.

Voice is off by default. Explicit model downloads contact **Hugging Face and its download hosts**; the models are optional and outside the installer. Small Q5_1 is 190,085,487 bytes; Base Q5_1 is 59,707,625 bytes. Microphone audio and recognition stay on your computer, with no account, billing, cloud rewriting or telemetry. Voice keeps no recording or transcript files and does not use the clipboard. Terminals that receive voice text skip their saved-screen file for that app run to avoid retaining its echo. The agent program still owns its own conversation history. [Voice guide](docs/guide.md#voice-input) · [Measured sizes and validation](docs/reviews/voice-validation-2026-10-09.md).

## Documentation

- [Guide](docs/guide.md): using the app, settings, and where its files are kept
- [Programs](docs/programs.md): the CLIs it knows, and adding your own in `harnesses.json`
- [Internal MCP](docs/internal-mcp.md): automatic desk naming, activity reports, delegated sessions and task results
- [Development](docs/development.md): building from source, how the code is laid out, tests
- [Known limits](docs/limits.md)
- [Code signing policy](docs/code-signing-policy.md), with what the app sends
- [DESIGN.md](DESIGN.md) and [PRODUCT.md](PRODUCT.md): how it looks, and who it is for

## Building from source

You need Node 22.12 or later, Rust 1.89 or later, the [Tauri prerequisites](https://tauri.app/start/prerequisites/), CMake and a C++ toolchain. Windows and macOS also need libclang to generate target bindings for the pinned Whisper crate; Linux can use its packaged bindings. Linux microphone capture needs ALSA development headers. Details are in [development.md](docs/development.md).

```sh
npm install
npm run tauri dev      # run from source
npm run tauri build    # make an installer for the system you are on
```

Tests and the layout of the code are in [development.md](docs/development.md).

## Code signing policy

The Windows installers are not signed yet. The project is applying for free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org). Who may sign, what is signed and what the app sends are in the [code signing policy](docs/code-signing-policy.md).

## Licence

[MIT](LICENSE). Geist and Geist Mono are bundled under the SIL Open Font Licence.

Voice uses whisper.cpp/ggml (MIT), whisper-rs (Unlicense) and CPAL (Apache-2.0). Optional Whisper model weights are MIT. See [third-party notices](THIRD_PARTY_NOTICES.md).

Agent Moshpit is not affiliated with Anthropic, OpenAI, Google, Nous Research or the makers of any other program it can run.
