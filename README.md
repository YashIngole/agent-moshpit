# Agent Moshpit

[![CI](https://github.com/YashIngole/agent-moshpit/actions/workflows/ci.yml/badge.svg)](https://github.com/YashIngole/agent-moshpit/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/YashIngole/agent-moshpit)](https://github.com/YashIngole/agent-moshpit/releases/latest)
[![MIT licence](https://img.shields.io/badge/licence-MIT-blue)](LICENSE)

A small desktop office for coding agents. Claude Code, Codex, Gemini CLI and the rest run as the real command-line programs, each in a terminal inside one window, and each is drawn as a person at a desk. One look tells you who is working, who is done, and who needs you.

![The Agent Moshpit window: agents at their desks on the left, three of their terminals in a grid beside them, and an amber band across the top naming the agent who needs you](docs/terminals.png)

**[Download](https://agentmoshpit.com)** · [Guide](docs/guide.md) · [Release notes](https://github.com/YashIngole/agent-moshpit/releases)

## Features

- **Real terminals.** Each agent is the `claude`, `codex` or `gemini` you already have, with your own sign-in and settings. The app draws none of the conversation and keeps no API key.
- **Who needs you.** When an agent stops on a question, a hand goes up at its desk, an amber band names it, the tray icon changes and you get a notification.
- **Side by side.** Open terminals in a grid beside the floor. Resize, rearrange and zoom them. [Every shortcut](docs/guide.md#keys) is listed in the app.
- **Stays out of the way.** Close the window and the agents keep working from the tray. After a restart every desk is back with its last screen, and Claude Code, Codex and Hermes carry on their conversations.
- **Thirteen CLIs, and yours.** Claude Code, Codex, Gemini CLI, Hermes, OpenCode, Copilot CLI, Amp, Qwen Code, Crush, Cursor CLI, Aider, Goose and Droid. Add another with [a few lines of JSON](docs/programs.md#adding-a-program-of-your-own).
- **Private.** No account, no telemetry. See [what it sends](#what-it-sends).

## Install

Get the installer for your system from [agentmoshpit.com](https://agentmoshpit.com) or the [latest release](https://github.com/YashIngole/agent-moshpit/releases/latest):

| System | File |
| --- | --- |
| Windows 11 | `agent-moshpit_windows_x64-setup.exe` (or the `.msi`) |
| macOS, Apple Silicon | `agent-moshpit_darwin_aarch64.dmg` |
| macOS, Intel | `agent-moshpit_darwin_x64.dmg` |
| Linux | `.AppImage`, `.deb` or `.rpm` |

The installers are not code-signed yet, so the first launch needs one extra step:

- **Windows:** on "Windows protected your PC", choose **More info**, then **Run anyway**.
- **macOS:** open **System Settings → Privacy & Security** and choose **Open Anyway**.
- **Linux:** `chmod +x` the AppImage before running it.

You also need at least one agent CLI, installed and signed in the way its makers describe. The app finds the ones you have and can install those published on npm.

## Quick start

1. Press **+ new agent** (or `n`), pick a program and a folder, and give it a task if you like.
2. The agent takes a desk and its terminal opens beside the floor. Type to it as you would in any terminal.
3. Hold `Ctrl` and click another desk to open it beside the first. `Ctrl` + `` ` `` goes back to the floor.
4. When a hand goes up, press **open their terminal** on the amber band and answer the question.

The [guide](docs/guide.md) covers the rest: the floor, terminals, the desk menu, notifications and every key.

## Platform support

| System | Status |
| --- | --- |
| Windows 11 | Used every day. Tested by hand and by a test that drives the real app. |
| macOS | Built and checked by CI. Not yet used by hand. |
| Linux | Built and checked by CI. Not yet used by hand. |

On macOS and Linux a few things are missing for now: a click on a notification does not open that agent's desk, the right-click **Paste** may ask before reading the clipboard, and the webview's own keys (reload, for one) are not turned off. If you run either build, please [report what breaks](https://github.com/YashIngole/agent-moshpit/issues). All known gaps are in [limits.md](docs/limits.md).

## What it sends

Nothing, with one exception: it asks npm for the newest version of each agent CLI you have installed, when it starts and every 12 hours. Set `MOSHPIT_NO_UPDATE_CHECK` to turn that off. The programs it runs talk to their own services as they always do.

## Documentation

- [Guide](docs/guide.md): using the app, settings, and where its files are kept
- [Programs](docs/programs.md): the CLIs it knows, and adding your own in `harnesses.json`
- [Development](docs/development.md): building from source, how the code is laid out, tests
- [Known limits](docs/limits.md)
- [DESIGN.md](DESIGN.md) and [PRODUCT.md](PRODUCT.md): how it looks, and who it is for

## Building from source

You need Node 22.12 or later, Rust 1.89 or later, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev      # run from source
npm run tauri build    # make an installer for the system you are on
```

Tests and the layout of the code are in [development.md](docs/development.md).

## Licence

[MIT](LICENSE). Geist and Geist Mono are bundled under the SIL Open Font Licence.

Agent Moshpit is not affiliated with Anthropic, OpenAI, Google, Nous Research or the makers of any other program it can run.
