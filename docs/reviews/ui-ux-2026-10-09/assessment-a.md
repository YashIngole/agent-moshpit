# Assessment A — independent design review

Date: 2026-10-09. Target: `src/App.svelte`, the whole desktop application. Baseline: unchanged frontend at HEAD `8f08ff6`. Other ongoing task edits are excluded from this review.

Method: independent Assessment A, source review plus live Playwright screenshots and interactions. No detector output or previous review findings were read. Parent had already run Impeccable context; it was not rerun. No application source changes were made.

## Design specificity verdict

Strongly authored for Agent Moshpit. The office metaphor serves the actual task: people, rooms, raised hands, sleeping agents, and lit screens make parallel CLI activity understandable. Geist/Geist Mono, graphite surfaces, restrained terminal chrome, and semantic status colors form a coherent system. Preserve this visual direction. The biggest opportunity is making its attention routing and workspace density hold up in smaller windows.

## Nielsen scores

| # | Heuristic | Score | Evidence |
|---|---|---:|---|
| 1 | Visibility of system status | 3 | Clear status words, poses, counts and attention band; narrow summaries and form validation lose visibility. |
| 2 | Match with real world | 4 | Office metaphor, folders, familiar terminals, and plain action labels fit the developer audience. |
| 3 | User control and freedom | 3 | Cancel/Escape restore focus; drafts survive dismissal; panes can be put away without stopping agents. |
| 4 | Consistency and standards | 3 | Cohesive controls and state language; ordinary desk clicks replace a pane while Ctrl-click adds one, requiring learning. |
| 5 | Error prevention | 3 | Busy destructive actions have confirmation; draft preservation and clear optional fields help. Missing-folder handling needs better placement. |
| 6 | Recognition rather than recall | 2 | Multi-pane and compact-floor features depend heavily on tooltips, modifier keys and the Keys panel. |
| 7 | Flexibility and efficiency | 3 | Arrow-key floor navigation, context menus, pane shortcuts, resizing and font controls provide good accelerators. |
| 8 | Aesthetic and minimalist design | 3 | Distinct, controlled visual system; excessive room width and default floor width waste useful workspace. |
| 9 | Error recognition and recovery | 2 | Stopped-agent recovery is clear; validation can appear outside the visible form viewport. |
| 10 | Help and documentation | 3 | Good field hints, empty-state teaching, tooltips and grouped keyboard reference. Secondary capabilities remain easy to miss. |
| **Total** | | **29/40 — Good** | Solid foundation with consequential visibility and layout gaps. |

These scores and the design verdict were recorded before detector evidence entered the review.

## What works

- The amber band creates a decisive next action and takes users into the CLI’s own screen. It disappears when that waiting terminal is visible, avoiding duplicate attention demands. `src/components/KnockBand.svelte:13,33`.
- Status is expressed through words, poses, symbols and colors. “Away” also explicitly says the program is not running; opening it offers **Carry on** without starting anything automatically. `src/components/Desk.svelte:99`; `src/components/Panes.svelte:366`.
- Escape preserves a partially completed New agent draft and returns focus. I filled task/folder/name, dismissed the panel, reopened it, and verified all three values. Floor arrow navigation, Shift+F10, menu arrows, and Escape focus restoration also worked.

## Priority findings

### 1. [P1] New-agent validation can be completely invisible

At 1024×700, submitting without a folder leaves focus on **Start agent**, with no visible explanation. The error is appended near the end of the scrolling form, below the viewport; the sticky footer remains visible.

- **Reproduce:** Open `http://127.0.0.1:1420/?demo=office&still` at 1024×700 → New agent → leave Folder empty → click Start agent.
- **Verified:** Error text was “Choose the folder the agent should work in.” Its bounding rectangle was `y=766…784`, while the window ended at `y=700`. Folder had no `aria-invalid`; focus remained on Start agent.
- **Source:** `src/components/NewAgent.svelte:62`, `:169`, `:202`, `:382`.
- **Evidence:** [a-new-error-1024.png](screenshots/a-new-error-1024.png) shows no error after submission.
- **Fix:** Put required-folder validation beside Folder, associate it through `aria-describedby`/`aria-invalid`, focus the invalid field, and scroll its error into view. Keep async start errors in a visible region immediately above the sticky footer.
- **Suggested command:** `/impeccable harden`.

### 2. [P1] Smaller windows cut the attention summary into unlabeled fragments

The header’s summary gives up width by clipping its contents. At 1024 with two panes, the trouble summary becomes a red dot and “1”. At 640 it can become only an amber dot. Other agents’ trouble/completion signals disappear while the floor is hidden.

The same count controls are deliberately excluded from keyboard and accessibility navigation, despite providing “find next agent in this state.”

- **Reproduce:** Office demo → open the waiting agent → Ctrl-click Refactor auth middleware → resize to 1024×700, then 640×800. Inspect the top summary. Tab through the header: counts are skipped.
- **Source:** `src/components/TopBar.svelte:38` uses `tabindex="-1"` and `aria-hidden="true"`; `:82`–`:92` implement clipping; `:138` removes the whole summary below 620.
- **Evidence:** [a-two-panes-1024.png](screenshots/a-two-panes-1024.png), [a-new-640.png](screenshots/a-new-640.png), [a-crowd-640.png](screenshots/a-crowd-640.png).
- **Fix:** Switch to a compact labeled status control before the full summary stops fitting; retain complete waiting/trouble counts, with a menu for all states. Expose its actions to keyboard and screen readers. Let branding/demo decoration yield earlier.
- **Suggested command:** `/impeccable adapt` plus `/impeccable audit`.

### 3. [P2] Crowd room sizing misses the intended two-column layout at 1280

In the 20-agent floor, every four-agent room occupies the full width, but its desks use less than half that width. Five rooms become five rows; the last eight agents are below the initial viewport. This is avoidable empty space, directly weakening the “glance at everyone” purpose.

- **Reproduce:** `http://127.0.0.1:1420/?demo=crowd&still` at 1280×800 → Back to the floor if remembered panes are open.
- **Verified:** Each room was 1238px wide and approximately 196px tall. Floor viewport height was 643px; scroll height was 1078px. Two room bases are approximately `612 + 612 + 16 = 1240px`, just exceeding the available 1238px after gutters/scrollbar.
- **Source:** `src/components/Floor.svelte:124`, `:169`, `:236`–`:246`. Its own comment at `:232` says two four-agent rooms should share a row at 1280.
- **Evidence:** [a-crowd-floor-1280.png](screenshots/a-crowd-floor-1280.png).
- **Fix:** Calculate crowd room width from available floor space, including gaps and scrollbar allowance, or use an adaptive room grid. Keep desk locations/status ordering stable.
- **Suggested command:** `/impeccable layout`.

### 4. [P2] The default floor consumes too much of a smaller multi-pane workspace

At 1024×700, the 420px floor leaves two terminal panes about 302px wide each. Pane names truncate, project context disappears, and terminal reading becomes heavily wrapped. The compact strip exists, but discovering a double-click on the divider is a weak route to a comfortable default.

- **Reproduce:** Office demo, default floor width → waiting terminal → Ctrl-click a second desk → resize to 1024×700.
- **Source:** `src/App.svelte:177`, `:182`, `:228`; `src/lib/office.svelte.ts:146`.
- **Evidence:** [a-two-panes-1024.png](screenshots/a-two-panes-1024.png).
- **Fix:** Use the compact floor as the initial layout for multiple panes in smaller desktop windows, while preserving an explicitly chosen width. Provide a visible “Compact floor” control or contextual divider hint.
- **Suggested command:** `/impeccable adapt`.
- **Limit:** Demo terminal output is synthetic; I verified narrow pane dimensions and header truncation, not how every real CLI redraws at that size.

## Cognitive load

Two of the eight checklist items fail: **chunking** and **minimal choices**. The ordinary floor has clear hierarchy, repository grouping and progressive disclosure. Load increases substantially in action sheets:

- New agent exposes five installed programs plus an install-expansion control.
- A running desk’s menu presented ten options in a single undivided stack.
- Terminal right-click adds up to four or five terminal actions to those desk actions.
- The app menu mixes programs, layout, preferences, shortcuts and quitting.

Group desk actions into session, folder, and destructive sections. Give Programs separate “Installed” and “Available” groups. Preserve the simple primary floor rather than adding more persistent controls everywhere.

## Emotional journey

The opening impression is welcoming and memorable; the empty state teaches the office in plain language. The peak is opening a person’s desk and immediately reaching a familiar CLI. Confidence drops when a smaller layout hides status information or Start agent appears to do nothing. The stopped-agent footer restores confidence with an explicit action and clear explanation. Good recovery copy should be retained.

## Persona red flags

- **Alex, power user:** At 1024, opening a second pane produces narrow terminals while the illustrative floor retains 420px. Compacting it requires knowing the divider gesture. Ctrl-click’s additive behavior is discoverable mainly through the empty-state copy, tooltip or Keys.
- **Sam, accessibility user:** Status counts are actionable for pointer users but hidden from assistive technology and skipped by Tab. Missing-folder validation neither focuses nor marks the field; its visible message can lie below the viewport.
- **Jordan, first-time user:** A ten-option desk menu requires distinguishing opening, adding, restarting, stopping and removing. Programs also truncates long install/update commands, reducing confidence about precisely what the action will run.

## Supporting observations

- Program command text is ellipsized without a visible expand/copy affordance or full-command tooltip: `src/components/Programs.svelte:219`–`:231`. Show the complete command on request.
- The Keys panel explains shortcuts well, but its largest groups contain 8–10 rows. Lead with the few shortcuts used for opening, switching and returning.
- The default seven-agent floor at 1280 puts the trouble agent below the first viewport. The header count helps pointer users find it; fixing accessible count navigation matters.
- Small crowd program tags truncate—for example Antigravity becomes “antigra…”—while status remains readable. This is a reasonable priority tradeoff, provided the full identity remains retrievable.
- No P0 task blocker was found.

## Scope and evidence

Frontend review is based on the unchanged surface at HEAD `8f08ff6`: `src/App.svelte`, relevant components, styles, product/design records and demo behavior. Other ongoing task edits are excluded from this baseline review. No source changes were made.

Screenshots were saved and visually inspected under `docs/reviews/ui-ux-2026-10-09/screenshots/a-*.png`. Coverage includes office/calm/empty/crowd; 1280×800, 1024×700 and 640×800; creation, validation, draft recovery, multi-pane layout, attention band, app/desk menus, keyboard navigation, Keys, Programs, and stopped-agent carry-on.

Native tray/notifications, real CLI execution, real install/update operations, and OS clipboard integration were not exercised. The `still` demo intentionally leaves newly created agents in Starting, so that was not treated as a product defect.

The browser session was closed after evidence collection to release the owned Playwright profile for Assessment B. No subsequent testing was performed while saving this assessment.

Questions skipped: parent requested a read-only assessment without user questions.
