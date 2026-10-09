# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Tauri 2: a Rust core plus the operating system's own webview, with the interface in TypeScript and Svelte 5. Chosen by the owner on 2026-10-05 from four offered options (Tauri 2, Electron, Flutter desktop, native Rust UI). Each agent's program runs in a pseudo-terminal held by the core (`portable-pty`) and is drawn in the window with xterm.js. One codebase for macOS, Windows and Linux; so far it has been built and run on Windows 11 only. The interface is HTML and CSS rendered in a desktop window, so the design language is web, not native.

## Users

Developers who run several coding-agent CLIs at once on their own computer: Claude Code and Codex first, and others such as Antigravity CLI and Hermes. The agents mostly work in different repositories or git worktrees. The user is busy in an editor or in one agent's terminal and looks over to see who is working, who needs an answer, and who has finished. Agent Moshpit is open source, not a private tool: it was published on 2026-10-09 at github.com/YashIngole/agent-moshpit, with a download page at agentmoshpit.com.

## Product Purpose

Agent Moshpit is a small desktop office for coding agents. Each agent is a real CLI running in a terminal inside the window, drawn as a person at a desk. The office shows every agent, who is working, who needs input and who is done, and puts the user in that agent's own terminal in one click. Success: the user never loses an agent to an unanswered prompt, can tell the state of every agent in one glance, keeps the full power of each CLI, and enjoys having the office open.

## Positioning

The office is not a harness. The first version was a client of Hermes that drew conversations, tool calls, diffs and approvals itself. On 2026-10-08 the owner decided to change that: rebuilding the coding inside the office lost the CLIs' own power, and the owner wants both the office view and the full Claude Code and Codex experience, in one window. So Agent Moshpit now starts the CLIs and shows each one's own screen. When a feature would mean drawing again something a CLI already shows, it is not built; the user is taken to the terminal instead.

The owner's requirements from the same day:

- One Agent Moshpit button in the Windows taskbar. No CLI may appear as a window of its own. Separate processes in Task Manager are fine.
- The reference for the terminal experience is BridgeMind's BridgeSpace: a sidebar of sessions and a grid of terminal panes, each a real CLI the tool started.
- A new screen is shown as a mockup before it is built.

Neighbours, as looked at in the two reviews of 2026-10-08 and 2026-10-09 (sources are listed there):

- Windows Terminal, the terminal in VS Code, and Warp are where the user's habits come from: panes, keys, copy and paste, dropping a file, clicking a path. The reviews compare the office with them feature by feature.
- Warp also tells the user when an agent needs them, with notifications in the app and from the system.
- BridgeSpace puts several agents side by side in a grid. It is known here only from third-party listings; no official documentation was found.
- What separates Agent Moshpit from all of them is the floor: who is working and who needs you, at a glance, drawn as people, for any CLI.

## Operating Context

- The user's own installed CLIs, with their own sign-ins, settings and models. The office knows fourteen by name and five in detail: Claude Code, Codex, Antigravity CLI, Hermes and Gemini CLI. Four were read from each program's `--help` on 2026-10-08 (Claude Code 2.1.290, Codex 0.160.0, Gemini CLI 0.2.1, Hermes 0.21.2); Antigravity CLI, which replaced Gemini CLI for personal Google accounts, was read from Google's documentation on 2026-10-09 and has not been run. A user can add a program, or replace a row, in `harnesses.json`.
- What the office shows but does not own: the conversation, approvals, slash commands, models, accounts, worktrees and history. All of that is the program's and is seen through its terminal.
- What the office does own: the desks (which program, its name, its folder, the id of its conversation when known), how the panes are laid out, the reading of each agent's status, and getting the user's attention (the band, the tray, notifications, the window's title).
- The desktop: a tray icon, system notifications, and a window that may be small and kept at the side of the screen, or closed entirely.
- Windows first.
- Typical load: 2 to 8 agents at once. The design must stay usable to about 20; the demo's `crowd` scene has 20 for checking this.

## Capabilities and Constraints

Confirmed by the owner:

- Low CPU and memory use, fast startup, simple UX and progressive disclosure are priorities.
- Experience and fun are not to be traded away for resource savings (owner, 2026-10-05). Efficiency comes from engineering, not from removing life from the office.
- The office does not rebuild what a CLI already does (owner, 2026-10-08).
- One taskbar button, and no CLI in a window of its own (owner, 2026-10-08).
- macOS, Windows and Linux from a single codebase.
- Open source. It must not read as official software from the maker of any program it runs. (The owner said this of Nous Research; the builder has extended it to the others.)

What follows from running real CLIs instead of a backend:

- Status is read, not reported. Claude Code says whether it is busy, idle or waiting in a file it keeps about each running session. Every other program is read from how its terminal behaves: printing, quiet, or ringing the bell. The office never reads the words on a screen, so a program is free to change them.
- The office cannot say what an agent is working on, only that it is.
- Whether a stopped agent carries on its conversation depends on the program: Claude Code, Codex and Hermes can, the others start afresh.
- The CLIs are children of the app. Closing the window leaves them running; quitting the app ends them.
- Only agents started from the office are shown.

Checked on the owner's Windows 11 machine:

- 2026-10-08, `spikes/cli-office/pty-proof/`: Claude Code and Codex run as children of one windowed app, each in a pseudo-terminal, with no window of their own and nothing left running afterwards.
- 2026-10-08 and 2026-10-09: two walks through the built app, with the fixes that followed each. What was checked, how, and what was not, is in `docs/reviews/`.

No performance figures have been measured for this version. The first version's figures were mostly measurements of Hermes and went with it.

Decided while building version 0.1.0 (2026-10-05 and 06). The owner delegated the first and left the other two open, so these are the builder's defaults and can be changed:

- How the user looks at the office: one adaptive window that also works as a narrow strip, with the tray and notifications taking over when it is closed.
- Licence: MIT.
- Accessibility target: WCAG 2.2 AA.

Decided while building the CLI office (2026-10-08 and 09), also the builder's defaults:

- Closing the window leaves the office in the tray. A setting makes it quit instead, added after the owner asked in the first review how to close the whole office.
- The × on a pane puts the pane away and leaves the program running. Ending a program is Stop, and taking a desk away is Remove, which can be undone for a few seconds instead of asking first, unless the agent is busy.
- Opening the desk of an agent that is not running shows where they left off and starts nothing.
- The office asks npm for newer versions of the installed programs, and GitHub for a newer version of itself, unless told not to. It sends nothing else anywhere.

No longer true: connecting models from inside the app (asked for by the owner on 2026-10-06 and built for the Hermes client) went with that client. Each program is signed in and set up its own way.

Open decisions:

- Whether to show sessions the office did not start (one typed into another terminal, or one inside a desktop app) and offer to carry them on here. It is in the plan (`docs/plan-cli-office.md`, step 6) and is not built.
- Whether an agent should outlive the app. Today quitting ends every program.
- What a desk says while its agent is working. Today it says only "Working".
- macOS and Linux: built and checked by CI since 2026-10-09, not yet used by hand.

## Brand Commitments

Name: Agent Moshpit (owner, 2026-10-05). It replaces the earlier working name Talaria, which several Hermes Agent clients on GitHub already use. A web search on 2026-10-05 found no other project called Agent Moshpit.

Visual reference from the owner: a virtual office like Gather, where each agent is a character at a desk (owner, 2026-10-05). For the terminals, BridgeSpace (owner, 2026-10-08).

As built, the office is drawn after hours: dark graphite rooms, people at desks lit by their screens, type in Geist and Geist Mono. Four colours carry meaning and are used for nothing else: amber for needs you, green for done, red for trouble, blue for a screen at work. A status is always a word and a pose as well as a colour. The record of this is `DESIGN.md`; the values themselves are in `src/styles/tokens.css`.

## Evidence on Hand

- `spikes/cli-office/pty-proof/`: the proof program and its result from 2026-10-08.
- `docs/plan-cli-office.md` and `docs/mockups/`: the plan for the CLI office and the mockups drawn for it before it was built. They are not pictures of the built app, and the plan differs from what was built in places.
- `docs/reviews/ux-sweep-2026-10-08.md` and `docs/reviews/ux-sweep-2026-10-09.md`, with their screenshots: two walks through the built app, flow by flow, each ending with what was changed and what was not checked.
- `docs/office.png`, `docs/terminals.png`, `docs/strip.png` and `docs/new-agent.png`: the built window showing its demo data, taken on 2026-10-09 for the README.
- `docs/first-real-terminals.png`: the first real Claude Code and Codex in panes, in an earlier, light look.
- `DESIGN.md`: the look as built, rewritten on 2026-10-09 from `src/styles/tokens.css` and the components. `.impeccable/design.json` is made from it by `tools/make-design-json.mjs`.
- From the first version, kept for the record and no longer describing the app: `docs/reviews/codebase-review-2026-10-06.md`, and the pictures `docs/office-day.png`, `docs/office-night.png`, `docs/room.png` and `docs/models.png`.
- `assets/brand/`: the icon drawings, made for this project.
- No testimonials or users yet. Future work must not invent them.

## Product Principles

1. The CLI does the work; Agent Moshpit shows who is doing what and routes attention. It never draws again what a CLI already shows.
2. Who needs you comes first. A raised hand stays up until the question is answered.
3. The office should be a pleasure to keep open. Efficiency is an engineering job, not a reason to make it dull.
4. Progressive disclosure: a glance at the floor, then the terminal. Each step only when asked, and opening a desk never starts anything.
5. No surprises: never steal focus, never end an agent's work silently, and make anything destructive either asked about or undoable. Say honestly who will carry on and who will start afresh.
6. What is typed in a terminal belongs to its program. The office keeps only a short, listed set of keys for itself.
