Agent Moshpit UI/UX fixes · 9 October 2026

All 15 findings from [the baseline review](../review.md) are addressed. The office's rooms, people, palette, stable seating and terminal workflows are preserved. These changes build on `342b0f2`, including its per-agent launch settings and live model catalogs. No new settings system, confirmation preference or redesign was added.

| Finding | Resolution | Verification |
| --- | --- | --- |
| UX01 | Added Quiet for unconfirmed inactivity. Done requires a completion signal. Quiet remains busy for Stop, Restart, Remove and Quit. Claude's explicit idle signal still confirms completion. | Rust state transitions; native terminal silence and Stop confirmation. |
| UX02 | Backend supplies desk, folder, latest or fresh resume scope. Actions, hints and restart/quit copy describe that scope; ambiguous continuation no longer promises this desk's conversation. | Adapter and wording tests; Antigravity and Hermes browser fixtures. |
| UX03 | Folder is required, receives focus on error and links to its visible inline error. Typing, Browse and recent-folder selection clear the invalid state. | Browser checks at 1024×700 and 340×420; launch-settings labels remain compatible. |
| UX04 | Waiting remains on the band until answered. Only the selected, uncovered terminal is watched; forms, menus and hidden/zoomed panes do not suppress attention. | Browser watched-state checks; native waiting and notification flows. |
| UX05 | Minimum-size band preserves agent identity and action; other waiting agents wrap onto their own row. | 340×420 identity width above 80px; no horizontal overflow. |
| UX06 | Desk-menu keyboard focus has an inset 2px focus outline. | Computed `rgb(231, 234, 240) solid 2px`; keyboard capture. |
| UX07 | Full status buttons participate in keyboard navigation. Narrow windows expose a labeled status button and menu using the existing menu component. | Keyboard status action opens the trouble terminal at 640px. |
| UX08 | Form fields scroll independently; footer actions occupy their own grid row. | Focused Name remains above footer at desktop and minimum size. |
| UX09 | Dense room bases account for actual desk, gap and edge widths. Narrow crowds use full-width rows for readable names. | Two rooms share a row at 1280×800; minimum-size nameplate is 184px. |
| UX10 | Two or more panes start with a compact floor below 1180px unless a width has been chosen. More exposes Compact floor, alongside the existing resize gestures. | Two 380px panes beside a 264px floor at 1024px; explicit expansion works. |
| UX11 | Persistent startup/storage warnings are separate from action errors. Navigation preserves them; explicit dismissal retains a recovery menu entry and file action. | Browser warning navigation, dismissal and reopening checks. |
| UX12 | Pending removals share Undo all. Every new removal renews the recovery window; hover or keyboard focus pauses expiry. Closing the notice explicitly finishes removals. | Two removals restore together after holding Undo focus beyond eight seconds. |
| UX13 | More scrolls within available window height. | End reaches Quit at 340×420 without scrolling the document. |
| UX14 | Install/update commands wrap in full and remain selectable. Menu hints also wrap without truncation. | Full-command geometry and latest-conversation hint regression. |
| UX15 | SpareDesk uses the radius token; the three 10.5px tags use the small text token. Design and guide describe the resulting behavior. | Source diff and clean Svelte checks. |

Supporting improvements: Agent programs separates Installed and Available; menu dividers group related actions; duplicate recent-folder basenames include parent context and full accessible paths; the terminal cursor follows reduced motion, including preference changes.

Validation completed:

- `npm run check`: zero errors and warnings.
- Production frontend and Tauri debug builds succeeded.
- `npm test`: 50 tests passed.
- Demo browser regression: 181 checks passed, including the new recovery, accessibility and layout checks.
- Launch-settings browser suite: all six behavior groups passed.
- Rust unit suite: 80 passed, one intentionally ignored live CLI catalog check.
- Clippy with warnings denied: passed.
- Native terminal/lifecycle and reliability suites: passed in an isolated named office with stand-in CLIs. Covered terminal I/O, Quiet safeguards, waiting state, notifications, session identity, persistence, launch settings and child-process cleanup.
- `git diff --check`: passed.

Visual evidence is in this folder. Final captures cover minimum-size attention, form editing, validation, More, program commands, keyboard focus, resume scope and enlarged text. The 200% sample enlarged the root text size; it was not a browser-zoom or screen-reader certification. No real model task, CLI installation/update, or real Antigravity/Hermes resume was performed. Physical touch and other operating systems were not exercised. The original review scores remain historical; this pass did not invent a new score.

The installed user office was left running. Review preview servers and browser tabs were closed. The earlier policy-blocked temporary baseline snapshot remains as recorded in [the cleanup note](../cleanup-note.md).
