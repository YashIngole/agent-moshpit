Method: dual-agent (A: /root/design_assessment · B: /root/technical_assessment)

Agent Moshpit UI/UX review · 9 October 2026 · baseline commit `8f08ff6`

**Verdict: preserve the visual direction; strengthen operational trust and recovery.** The office is authored for this product. Rooms, people, raised hands, sleeping agents and familiar terminals help developers understand parallel work. The most consequential gaps concern whether status is trustworthy, whether waiting agents stay visible, and whether failed actions give users a clear next step.

This review covers the whole desktop frontend, anchored to `src/App.svelte`, with source inspection of the backend contracts that drive its labels. Concurrent launch-settings changes are excluded. Neither assessment changed application code. A recorded its independent design judgment before B's detector findings entered the synthesis. The synthesized score is lower than A's 29/40 because the parent source/recovery pass found additional status-confidence and destructive-guard risks.

**Design health**

| # | Nielsen heuristic | /4 | Main evidence |
|---|---|---:|---|
| 1 | Visibility of system status | 2 | Excellent status language, but quiet can mean Done and covered panes still count as watched. |
| 2 | Match with the real world | 4 | People, desks, rooms, folders and real CLI screens fit the developer task. |
| 3 | User control and freedom | 3 | Escape and drafts work; a second removal replaces the first Undo. |
| 4 | Consistency and standards | 3 | Cohesive controls; Carry on promises more than some adapter resume targets support. |
| 5 | Error prevention | 2 | Busy confirmations exist, but inferred Idle/Done can bypass them. |
| 6 | Recognition rather than recall | 2 | Compact floor and additive panes rely on gestures, tooltips or Keys. |
| 7 | Flexibility and efficiency | 3 | Strong floor keyboard navigation, pane controls, resizing and terminal shortcuts. |
| 8 | Aesthetic and minimalist design | 3 | Distinct, restrained composition; room sizing and floor allocation waste workspace. |
| 9 | Error recognition and recovery | 2 | Helpful copy, but validation is offscreen and unresolved startup warnings disappear. |
| 10 | Help and documentation | 3 | Useful field hints and Keys; full program commands and resume scope need better exposure. |
| **Total** | | **27/40 — Acceptable** | Significant behavioral improvements needed; the visual foundation is strong. |

These are expert heuristic judgments, not usability-test completion rates.

| Technical audit dimension | /4 | Assessment |
|---|---:|---|
| Accessibility | 2 | Good labels and floor navigation; weak menu focus and hidden functional controls. |
| Performance | 3 | Shared animation beat and terminal safeguards; native load unprofiled. |
| Responsive design | 2 | Narrow variants exist, but attention identity, form fields and menus can be clipped. |
| Theming | 3 | Coherent dark tokens and deliberate illustration colors. |
| Implementation integrity | 3 | Product-specific composition; most automated style warnings are intentional. |
| **Total** | **13/20** | |

**What works**

- The office metaphor carries information. Status uses words, poses, symbols and color together; Away explicitly says the program is not running. The floor feels like a place to coordinate work.
- The attention band takes users straight to the CLI's own permission or question screen. This keeps the user's decision close to the actual program.
- Escape restores focus and preserves task, folder and name drafts. Arrow navigation, Shift+F10, menu arrows and pane controls worked. Opening an away desk shows its snapshot without automatically starting a process.

**Five priorities, in order**

1. **[P1] Make status and resume claims match what the app knows.**

   Two source-supported risks matter more than cosmetic changes.

   First, generic terminal activity becomes Quiet after 2.5 seconds without output. The office handles Quiet and explicit Finished together: an unseen agent that previously worked can become Done; a watched one can become Idle. Quiet long-running work can consequently disappear from the “busy” set used by Stop, Restart, Remove and Quit safeguards. Claude's session-file adapter provides stronger information when available, so this risk does not apply identically to every program.

   Sources at this commit: `src-tauri/src/status.rs:33,149`; `office.rs:129,279`; `engine.rs:417,543`; `src/lib/office.svelte.ts:725`. Preserved excerpts are in `backend-source-excerpts.json`. This is a code-level risk, not a measured real-CLI false-completion rate.

   Second, the adapters configure Antigravity to continue the latest conversation in its folder and Hermes to continue its latest conversation, whichever desk produced it. The UI broadly says “Carry on” or “It carries on the conversation”; it does not expose that different scope. The repository documents the limitation, but the action itself suggests desk identity. Sources: `src-tauri/src/harness.rs:181,189,399`, `office.rs:418`, `src/lib/menus.ts:23`, `src/components/Panes.svelte:366`, `docs/programs.md`. Real CLI resume was not executed.

   **Change:** distinguish confirmed completion from inactivity and expose status confidence where it affects a decision. Base destructive safeguards on live processes whose work state is uncertain, with an explicit user preference for repeated confirmations. Represent resume capability as exact desk, latest in folder, latest overall, or fresh start; show the correct target before restarting/waking.

   **Acceptance:** a quiet process is never labeled confirmed Done solely because it stopped printing; restart copy names the actual resume scope. Suggested commands: `/impeccable harden`, `/impeccable clarify`.

2. **[P1] Keep the attention system dependable across layouts and input methods.**

   At the supported minimum 340×420, the crowded office gives the attention band's identity area a measured width of zero. Its queue extends to x=354.75, outside the 340px viewport. Users can see an action while losing who it concerns. Source: `src/components/KnockBand.svelte:165`; evidence: `b-crowd-minimum-layout.json`, `b-crowd-minimum-340x420.png`.

   Opening a waiting terminal removes it from the band. Opening New agent over that terminal does not remove it from `watched`. At 640×700, the panel covered the terminal, the waiting band remained absent, and the demo bridge still reported `demo-1` watched. The native notification code also suppresses a watched agent's notification while the app is focused; that consequence is inferred from source, not tested through native notifications. Sources: `KnockBand.svelte:13`, `office.svelte.ts:595`, `src-tauri/src/engine.rs:596`; evidence: `c-waiting-pane-covered.png`, `scope-and-parent-evidence.json`.

   The top summary additionally clips to unlabeled fragments at 1024/640 and is hidden below 620. Its find-next-state buttons use `tabindex=-1` and `aria-hidden`; a polite text summary does not supply equivalent actions. Floor arrow navigation is a fallback, so the hidden controls are a P2 efficiency/accessibility finding. Source: `TopBar.svelte:38,82,138`.

   Keyboard focus inside desk menus has no outline and only an approximately 1.10:1 fill against the menu. This is a P1 focus-state contrast defect. Source: `Menu.svelte:161`; evidence: `b-menu-focus.json`, `b-menu-keyboard-focus.png`. Restore a state cue with at least 3:1 contrast against its adjacent background. [W3C non-text contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).

   **Change:** retain outstanding waiting state until it is answered; distinguish open, actually visible, and focused panes. Give the band a stable identity slot and move the queue to a second row or compact counted menu. Expose a compact labeled attention/trouble control to keyboard and assistive technology. Restore a visible inset focus ring.

   **Acceptance:** every supported window size shows who is waiting, and a covered waiting pane remains reachable through a visible cue. All pointer status actions have keyboard equivalents. Suggested commands: `/impeccable adapt`, `/impeccable harden`.

3. **[P1] Keep New agent errors visible and associated with the field.**

   At 1024×700, submitting an empty Folder leaves focus on Start agent while the error is at y766–784, entirely below the viewport. The sticky action row remains visible, making the button seem ineffective. Folder lacks requiredness/error association and is not marked invalid. Sources: `NewAgent.svelte:62,169,202`; evidence: `screenshots/a-new-error-1024.png`.

   A separate P2 issue affects editing: at 1280×800, focused Name spans y724.89–760.48 while the sticky footer starts at743, masking the lower part and text. Task is partially covered at 340×420. A stable fully obscured focus state was not established, so this review does not assert a WCAG2.4.11 failure. Source: `NewAgent.svelte:382`; evidence: `b-panel-focus-name.png`, `b-minimum-focus-obscured.json`.

   **Change:** put the Folder error beside Folder; mark requiredness, `aria-invalid` and its error association; focus/scroll to the invalid field. Put actions outside the form's scrolling area and reserve space for asynchronous failures above the actions.

   **Acceptance:** submitting without Folder makes its explanation immediately visible; focused fields remain clear of the footer at all supported heights. Preserve existing draft recovery. Suggested commands: `/impeccable harden`, `/impeccable clarify`, `/impeccable adapt`.

4. **[P2] Allocate space around terminal reading and whole-floor awareness.**

   At 1024×700, the default 420px floor leaves two panes approximately 302px wide each. Titles truncate, project/program context disappears and terminal text wraps heavily. Compact floor exists, but discovering a divider double-click is a weak default recovery path. Sources: `App.svelte:177,182,228`, `office.svelte.ts:146`; evidence: `screenshots/a-two-panes-1024.png`.

   With 20 agents at 1280×800, four-agent rooms stack as full-width rows and leave large unused areas. Two 612px room bases plus a 16px gap total 1240px, just exceeding the1238px available width. Floor scroll height is 1078px against 643px visible; the last eight agents fall below the initial viewport. Sources: `Floor.svelte:124,169,236`; evidence: `screenshots/a-crowd-floor-1280.png`.

   **Change:** choose a compact floor initially for multiple panes in smaller desktop windows, preserve the user's explicitly chosen width, and provide a visible compact/expanded control. Size crowd rooms from actual available space including gutters/scrollbar; preserve stable seating so agents do not constantly move.

   **Acceptance:** two panes at 1024 start with useful terminal width, and the crowd's intended two-room layout fits at 1280 without deleting room identity. Suggested commands: `/impeccable adapt`, `/impeccable layout`.

5. **[P2] Let recovery remain available until the user finishes it.**

   In the `problem` fixture, opening an ordinary desk clears the unresolved harnesses.json startup warning: one alert before the click, zero afterward, without dismissal or repair. `office.show()` resets the shared problem field, including its file-recovery information. Sources: `office.svelte.ts:259,281,376`; evidence: `c-startup-warning-cleared.png`, `scope-and-parent-evidence.json`.

   Removing two nonbusy desks within the 8-second Undo period leaves only the second Undo. Clicking it restores the second desk; the first remains removed. The first pending removal still runs when its timer expires. Sources: `office.svelte.ts:69,735,800`; evidence: `c-undo-overwritten.png`, `scope-and-parent-evidence.json`.

   **Change:** keep persistent health/storage errors separate from transient action errors. Resolve them through repair or explicit dismissal and retain a recovery entry. Give each pending removal an Undo, or provide one explicit batch Undo for all pending removals; pause expiry while the recovery control is focused/hovered.

   **Acceptance:** desk navigation cannot silently remove a configuration warning; consecutive removals remain recoverable during their advertised Undo periods. Suggested command: `/impeccable harden`.

**Complete finding register**

Severity counts are0 P0, 6 P1, 7 P2, 2 P3. Several individual findings are grouped into the five priorities above; automated advisories are not counted as separate user defects.

| ID | Severity | Finding | Evidence class |
|---|---|---|---|
| UX01 | P1 | Quiet can become Done/Idle and weaken destructive guards | Source-supported risk; native CLI behavior untested |
| UX02 | P1 | Desk-specific resume wording exceeds some adapter scopes | Source/documented configuration risk |
| UX03 | P1 | Required-folder error is offscreen after submit | Browser reproduced |
| UX04 | P1 | Covered waiting pane still counts as watched | Browser reproduced; notification consequence inferred |
| UX05 | P1 | Minimum-size band loses agent identity | Browser measured |
| UX06 | P1 | Desk-menu keyboard focus fill has low contrast | Browser measured + CSS |
| UX07 | P2 | Status summary clips; functional counts hidden from keyboard/AT | Browser/source |
| UX08 | P2 | Sticky footer partly masks focused fields | Browser reproduced |
| UX09 | P2 | Crowd room sizing misses intended two-column fit | Browser measured |
| UX10 | P2 | Default floor crowds two laptop panes | Browser measured |
| UX11 | P2 | Ordinary navigation clears unresolved startup warning | Browser reproduced |
| UX12 | P2 | Subsequent removal replaces earlier Undo | Browser reproduced |
| UX13 | P2 | More menu exceeds supported minimum height | Browser reproduced |
| UX14 | P3 | Program commands truncate without full-command affordance | Browser/source |
| UX15 | P3 | Four minor token/documentation discrepancies | Detector/source verified |

For UX13, at 340×420 the More sheet extends below the viewport. With the update fixture, pressing End to Quit scrolls the whole document 66px and moves the sheet top to −22px. Clamp the menu to available height and scroll internally, as the desk menu already does. Source: `AppMenu.svelte:192`; evidence: `b-more-update-minimum-340x420.png`.

For UX14, expose complete install/update commands on request through expansion or copying; keep the short default rows. Source: `Programs.svelte:219–231`.

For UX15, reconcile SpareDesk's 9px plus radius and three 10.5px tags with the design vocabulary, or document them. These are low impact and should follow behavioral fixes.

**Cognitive load, emotional journey and personas**

Two checklist areas fail: chunking and minimizing simultaneous choices. New agent offers five installed programs plus an installation branch; desk menus contain roughly 9–10 actions, and terminal context menus add another 4–5. This is not a reason to impose an arbitrary four-option limit on a developer tool. Group actions into Session, Folder and Destructive sections, keep the common action first, and separate Installed from Available programs.

The opening impression is welcoming. The strongest moment is moving from a recognizable person to a familiar CLI. Confidence drops when Start appears inert, a waiting agent's cue vanishes, or Carry on's target is uncertain. Draft preservation and explicit stopped-agent recovery are reassuring patterns to retain.

- **Alex, power user:** adding a second pane at 1024 leaves cramped terminals; the compact-floor gesture requires recall.
- **Sam, keyboard/low-vision user:** floor navigation works, but actionable counts are skipped, menu focus is faint, and required-folder recovery does not guide focus. This was a keyboard/DOM review, not an NVDA usability session.
- **Jordan, first-time user:** start errors can appear invisible; long ungrouped action menus and truncated commands make high-stakes decisions less predictable.

**Detector and accessibility evidence**

The baseline CLI detector returned 30 advisories: 10 radius, 8 typography, 11 color and 1 grid. Individual validation dismissed 26 as documented/illustrative intent and retained four minor discrepancies. The office grid and person's palette belong to the chosen world.

Four successfully injected browser views emitted 18 repeated finding events. Seven terminal low-contrast events describe xterm's invisible transparent accessibility scaffolding, not its canvas text. Crowd overflow did correspond to the real zero-width band identity. The overlays were captured and closed; no live overlay is left running.

All 56 sampled direct-text office elements reached 4.5:1; the lowest measured ratio was 4.87:1. This excludes SVG illustration text and xterm canvas and is not a full WCAG certification. [W3C text contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).

Reduced-motion emulation stopped the shared pose beat and opened the panel visibly; the suspected frozen-fade failure was not reproduced. xterm's `cursorBlink:true` is not tied to the preference in code, but real cursor behavior was not verified. Dynamic terminal/WebGL chunks, a shared 320ms beat, hidden-window stop,60ms resize debounce and stable pane identity are useful safeguards. The terminal import begins when Panes is instantiated; do not treat it as a strictly first-open lazy load without measuring.

**Scope and validation**

Inspected office, calm, empty,20-agent crowd, New agent, validation/draft recovery, terminal panes, away/error states, Programs, Keys, desk/terminal/app menus and update/configuration fixtures. Viewports ranged from 340×420 through 640/1024 to 1280×800. The existing demo regression suite passed 156 checks on an isolated port after a transient preview-server failure; only the temporary runner's port was changed, and that file was removed.

No real CLI launch/resume/install/update, native notifications/tray, native WebView, physical touch, screen-reader session,200% zoom or CPU/memory profile was exercised. Backend status/resume findings are labeled risks rather than observed real-program failures. The initial snapshot font403s were preview setup errors and were corrected before final B screenshots. All review-owned browsers, detector and Vite servers were closed/stopped.

The archive includes both independent assessments, raw/validated detector output, contrast/layout JSON, screenshots and parent recovery evidence. `baseline-App.svelte` preserves the reviewed target bytes; current workspace launch-settings edits are excluded.

**Questions for a follow-up:** Which priority should lead: (a) status/attention/recovery, (b) terminal space/layout, or (c) both together? Which setup should guide density: (a) 2–4 agents on a laptop, (b) 8–20 agents on a larger display, or (c) both equally?

