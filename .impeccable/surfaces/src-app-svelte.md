---
version: 1
slug: "src-app-svelte"
primary_target: "src/App.svelte"
related_targets: []
---

# The office window

Scope: the main window of Agent Moshpit (`src/App.svelte` and everything it draws). Visitor mode: Operate.

Audience and job: a developer running several coding-agent CLIs at once (Claude Code and Codex first, others such as Antigravity CLI and Hermes), busy in an editor or in one agent's terminal, who looks over for a second. They must see who is working, who needs them and who is done, and be in that agent's own terminal in one click.

Content it carries: the desks the office keeps (which program, the agent's name, its folder and branch), the status the office reads for each (needs you, working, done, idle, trouble, starting, away) and how long it has been so, and the programs' own terminals. The office cannot say what an agent is working on, only that it is. Two to eight agents is typical; twenty must still work.

Constraints: a Tauri webview on three engines, so conservative CSS; built and run on Windows 11 only so far. The office never draws again what a program already shows, and never reads the words on a program's screen. What is typed in a terminal belongs to its program; the office keeps a short, listed set of keys. Nothing moves while the window is hidden or the system asks for less motion; it keeps moving while unfocused, because that is when someone glances over at it. Every status is also a word and a mark, never colour or pose alone. WCAG 2.2 AA is the target. No remote fonts or assets.

## Direction contract

This contract was rewritten on 2026-10-09 from the built app, after the window stopped being a Hermes client and became an office over real CLIs. No direction round is recorded for the dark look: it appears first in the `night-` mockups (`docs/mockups/night-1-floor.png`, `night-2-terminals.png`, `night-3-cast.png`) and was settled in the build. The first version's contract is in this file's history and under the git tag `v0.1.0-hermes`.

THESIS: Your agents are colleagues in a room, and each one's work is in its own terminal. One look at the floor says who is heads-down, who has a hand up and who is finished, and one click puts you in that agent's own program. It refuses the status table of agent dashboards, and it refuses to be a harness: there is no conversation, no diff and no Approve button of the office's own.

OWN-WORLD: The office after hours, in flat vector. Graphite rooms on a faint grid, matte desks, people told apart by skin, hair, clothes and what is on their desk, and light that comes only from screens. Amber belongs to "needs you", green to done, red to trouble, blue to a screen at work. Geist sets names and sentences; Geist Mono sets the office's own signs, status, tags, buttons and keys. Terminals are black glass in the sixteen colours a program asks for. Controls are ordinary buttons, fields, menus and one side panel.

STORY: You glance over and the room is dark and calm, except one person waving under an amber "?". The same amber runs across the top of the window with their name and one button. You press it, their terminal opens beside the floor, and you answer the program itself; the hand goes down and they type again. Later someone leans back under a green tick, and you open their terminal to see what they did.

FIRST VIEWPORT: 1280 by 800. A 52-pixel bar: the mark and "agent moshpit", counts of only what wants a look (done, in trouble) with their dots, and "+ new agent", the microphone and a menu at the right. When someone needs you, a full-width amber band under the bar with their face, their name, which program is waiting and where, and "open their terminal". The floor fills the rest: one room per project with its folder name as a sign and a + beside it, desks in columns at least 180 wide, each with its person, a name, a status line, and a line only when the agent or its program said something the status does not. Opening a desk puts its terminal to the right of a 420-pixel floor; several open as a grid of panes, and the floor can be dragged down to a strip of names. The New agent form is a 380-pixel panel on the right. Under 760 wide the terminals or the panel take the window.

FORM: The category standard for the floor, named by the owner as "a virtual office like Gather", and BridgeSpace's sidebar of sessions beside a grid of terminals for the rest (kind: canon, the standing exit). Signature interaction: the hand goes up. An agent that needs you raises an arm and waves, an amber tag pops over its desk, its screen turns amber and the band drops in from above in the same colour. Motion grammar: state changes take 180 to 240 ms on an exponential ease-out; people loop in two to four fixed poses on a shared 320 ms beat; a new agent walks in once.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## As built (2026-10-09)

`DESIGN.md` was recorded from the build on 2026-10-09: every colour, radius and spacing step in it is checked against `src/styles/tokens.css` by `tools/make-design-json.mjs`, which also writes `.impeccable/design.json`.

No finish review in this skill's sense has been run on the CLI office, and no reviewer subagent. What it has had is two walks through the built app, flow by flow, with the fixes that followed each: `docs/reviews/ux-sweep-2026-10-08.md` and `docs/reviews/ux-sweep-2026-10-09.md`, with their screenshots. The four pictures in `docs/` (`office.png`, `terminals.png`, `strip.png`, `new-agent.png`) are the built window showing its demo data.

The app icon, the tray icons and the favicon were redrawn for this look on 2026-10-09, from the mark in the top bar; until then they were the first version's teal tile and oak desk. They were shown first as a mockup, `docs/mockups/night-4-icons.png`, and built after a yes to it. The PNG, ICO and ICNS files in `src-tauri/icons/` are rendered from the SVGs in `assets/brand/` by `tools/make-icons.mjs` and `tauri icon`.

Where the build differs from the mockups it was drawn from:

- The band names who is waiting and which program, not the command or the question. The office does not read a program's screen, so the question stays in the terminal and the band has one button.
- A desk at work says only "Working". The mockups showed the file being edited; the office has no way to know it.
- Sessions started outside the office are not shown (mockup 5).

Known and left as they are:

- Button labels are lowercase in the bar, on the band and in Agent programs, and in sentence case in the New agent form, in menus and in notices.
- The title of the New agent panel is weight 750; the other two panels' titles are 650.

## The smart office (2026-10-12)

A critique on 2026-10-12 (`.impeccable/critique/2026-10-11T18-49-30Z__src-app-svelte.md`) found the drawings carrying the status and the words around them repeating it. The owner asked for a smarter, calmer office and chose every change offered. Version 0.6.0 is the result:

- A desk's line under its name appears only when it says something new; the time comes before the program tag, and the tag only when more than one program is at work.
- The empty desk in each room became a + on the room's sign; head counts went.
- While terminals are open the band is an amber chip in the bar, so terminals never move when a hand goes up. Ctrl+Shift+A, or the chip, goes to whoever needs you, then the next.
- A plain click never puts away a pane whose agent needs you or is in trouble, and opens beside the others when they are wide enough. Two terminals or more, or a crowd, bring the strip of names by themselves.
- New agent asks who, what and where, with the folder filled in, and folds name, model and permissions into one line.
- Menus are one line an item, with keys at the end and hints only where something ends or is kept; settings are tick boxes.
- The microphone is an icon at rest.

## Open decisions

From `PRODUCT.md`, where they touch this window:

- What a desk says while its agent is working. Since 0.6.0 it says what the agent reported through the office's MCP tools (`set_activity`), and nothing when it reported nothing.
- Whether to show sessions the office did not start, and offer to carry them on here.

Carried over from the first version's brief and not looked at again since:

- Sounds (off by default if added).
- Things in a room besides desks (a door, a plant on the floor, a whiteboard). One attempt at a door read as a drawing error and was removed.
- Whether Hermes pets can stand in for the built-in people.
