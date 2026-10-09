# Assessment B: technical audit and detector evidence

Assessment B ran independently of Assessment A. Its detector/source findings were withheld until the parent explicitly confirmed Assessment A had finished. Frontend scope: commit `8f08ff6`, reviewed on 2026-10-09. Backend and frontend work occurring concurrently after this baseline is excluded. No application source was changed.

## Method and limits

Bundled `impeccable detect --json src` ran against the baseline source and is saved in `detector.json`; every finding is individually classified in `detector-validated.json`. After A released its standalone Playwright profile, B created its own fresh tab. Runtime inspection used Chrome 154 on Windows through standalone Playwright MCP and the frozen baseline Vite preview at `http://127.0.0.1:17430/`. The frozen preview initially denied two font files resolved through a node_modules junction. Parent corrected preview file access; final screenshots were retaken after `document.fonts.ready` and both Geist families checked loaded. Those initial403s are preview setup errors, excluded from product findings.

Browsed office, New agent, terminal, crowd, minimum340×420 and update-menu states. Four detector injections succeeded, with `[Human]` labels and screenshots/console logs. The detector visualization ran on B-owned PID48888, port28430; B stopped it and closed its owned MCP browser before reporting. No live overlay remains available after cleanup.

This is a web-rendered frontend audit, not a native Tauri verification. Real CLI launch/update, PTY input, native window/tray/notification behavior, native WebView rendering and macOS/Linux are untested by B. No screen-reader session, physical touch or synthesized touch gesture,200% text/zoom run, or formal CPU/frame/memory profile was completed. Browser viewport emulation establishes narrow layout evidence, not native-window behavior or touch support.

## Audit scores

| Dimension | /4 | Evidence |
|---|---:|---|
| Accessibility |2|Clear labels, screen-reader terminal mode and focus restoration coexist with weak menu focus, hidden functional status links and obscured fields.|
| Performance |3|Shared320ms pose beat, hidden-window stop, dynamic terminal/WebGL imports and60ms resize debounce. Native load and20-agent costs unmeasured.|
| Responsive design |2|Useful viewport/container queries; amber identity collapses in the crowd at the shipped minimum, footer masks fields, menu can exceed viewport.|
| Theming |3|Coherent dark token system; deliberate illustration palette. Four minor off-vocabulary values and a few untokenized artistic shades. Light theme is not a product requirement.|
| Implementation integrity |3|Product-specific coherent system; most detector advisories reflect documented intent rather than generic composition.|
| Total |13/20|Acceptable; significant work remains in focus and narrow layouts.|

Severity totals for the seven grouped findings below: P0=0, P1=2, P2=4, P3=1. These are technical review groups, not30 detector advisories promoted into30 user problems.

## Prioritized findings

1. **P1: the narrow attention band loses who needs the user.** `src/components/KnockBand.svelte:165` makes the band nowrap; `.who` grows/shrinks while `.answer` and `.queue` remain inflexible (`:174`,`:181`,`:191`). At340×420 in crowd, `.who` measured width0; queue right edge354.75px exceeds viewport340. The current agent's title disappears while an actionable button remains. Body scrollWidth355 with hidden overflow clips the queue. This breaks the primary glance-to-agent relationship. Give identity a minimum share; move queue to a second row or a counted menu, shorten the narrow CTA, allow wrapping. `/impeccable adapt`. Evidence: `b-crowd-minimum-layout.json`, `b-crowd-minimum-340x420.png`.

2. **P1: desk-menu keyboard focus is too faint.** `src/components/Menu.svelte:161` replaces the global2px focus outline with only an inset fill. Runtime focused menu item background#1b1f25 against menu#13161a is about1.10:1; computed outline style is none. This is a low-contrast focus-state cue (WCAG1.4.11), especially difficult in the deliberately dark palette. Keep a high-contrast inset ring or another3:1 state boundary; avoid relying on the slight fill. `/impeccable harden`. Evidence: `b-menu-focus.json`, `b-menu-keyboard-focus.png`.

3. **P2: sticky form actions cover the field being edited.** `src/components/NewAgent.svelte:382` places `.end` inside the scrolling form, sticky at a negative bottom offset, without reserving its height or scroll-padding. At1280×800, focused Name bounds y724.89–760.48 overlap footer y743–800; its center hit-tests to `.end`, and the visible screenshot masks the field text. At340×420, Task bounds296.14–393.33 overlap footer363–420; the lower part is covered. B did not confirm a stable fully-obscured focus failure, so this is not asserted as a2.4.11 AA violation. Move actions to a separate grid row outside the scroll area; reserve bottom space and add scroll-padding/scroll-margin as needed. `/impeccable harden` then `/impeccable adapt`. Evidence: `b-panel-focus-name.png`, `b-minimum-focus-obscured.json`, `b-new-agent-minimum-focus-340x420.png`.

4. **P2: top-bar More menu can exceed the minimum-height window.** `src/components/AppMenu.svelte:192` gives the sheet width but no viewport max-height or internal overflow. At340×420 its default sheet bottom494.55px; with `&update`, End to Quit scrolls the entire document66px and moves the menu top to−22.34px, bottom497.21px. Keyboard can bring Quit into view, but this displaces the app and hides the menu's start; pointer users lack a reliable internal scroll area. Clamp height to available space and make the sheet scroll, as the desk menu already does in `Menu.svelte:133`. `/impeccable adapt`. Evidence: `b-more-minimum-340x420.png`, `b-more-update-minimum-340x420.png`.

5. **P2: status counts expose pointer navigation but hide that function from keyboard/AT.** `src/components/TopBar.svelte:38` puts actual find-next-status buttons at tabindex−1 and aria-hidden. The polite summary supplies the words, but not the action. Floor arrow navigation works and can reach all agents, so this is an efficiency/discoverability issue rather than inability to open a terminal. Expose meaningful buttons with names such as “Next agent who needs you,” or provide a documented keyboard equivalent. `/impeccable harden`.

6. **P2: missing-folder recovery is detached from the field.** `src/components/NewAgent.svelte:62`,`:169`,`:202`: submitting an empty Folder produces a clear role=alert message, but leaves focus on Start; Folder has no required, aria-required, aria-invalid or error association. Identify requiredness before submission; focus Folder and associate its error/hint. Preserve the helpful existing error language. `/impeccable harden` and `/impeccable clarify`.

7. **P3: reconcile four minor token/documentation discrepancies.** SpareDesk plus radius9px at `src/components/SpareDesk.svelte:53`;10.5px tags at `Desk.svelte:333`,`:374` and `NewAgent.svelte:331`. Use the nearest named token or record the deliberate dense-tag step. This is low user impact; do not remove illustrative variation to satisfy detector output. `/impeccable document` followed by `/impeccable polish` after functional fixes.

## Detector verification

CLI: **30 advisory findings**,4 rule names,0 detector errors/blocking findings.

| Rule | Count | Verification |
|---|---:|---|
|design-system-radius|10|9 dismissed: DESIGN.md Shapes explicitly permits4–5px inside controls;1 minor drift at SpareDesk9px.|
|design-system-font-size|8|4 Person SVG glyphs are illustration,1 TopBar15px is explicitly documented;3 dense10.5px tags are minor vocabulary drift.|
|design-system-color|11|10 Person object/code/lamp colors and1 amber-action hover tone are deliberate. Document/tokenize shade additions; no demonstrated broken state semantics or readable-text contrast failure.|
|codex-grid-background|1|Dismissed: actual office-floor metaphor; DESIGN.md explicitly specifies the28px room grid.|

Thus26 findings are documented/illustrative intent and4 minor discrepancies. `detector-validated.json` preserves every exact file/line/snippet and verdict. Radius locations: Desk267, Keys187, KnockBand103, NewAgent296/329/408, Panes540, Programs271, SpareDesk53, base.css262. Type locations: Desk333/374, NewAgent331, Person462/584/612/615, TopBar77. Color locations: KnockBand128 and Person437/480/483/486/511/515/520/525/528/531. Grid: Floor168.

Browser overlay console finding events across four views (repeated events, not unique issues): **18**.

| View | Events |
|---|---|
|Office|1 grid-background|
|New agent|1 grid-background,1 flat-type-hierarchy|
|Terminal|1 grid-background,2 clipped-overflow-container,7 low-contrast|
|Crowd minimum|1 grid-background,2 clipped-overflow-container,2 text-overflow|

Grid and flat-type advisories are false positives for this authored, dense Operate surface: room headings, app wordmark and panel title have different roles, and New agent title is20px. The seven terminal black-on-black warnings concern invisible xterm accessibility text, whose vendor CSS explicitly uses `color: transparent` (`node_modules/@xterm/xterm/css/xterm.css:139–147`); canvas-rendered text is separate. Clipped xterm helper/positioned layers are implementation scaffolding and not evidence of lost visible task content. The crowd's two text-overflow events accompany a real band layout failure; its queue ellipsis alone is intentional, but the zero-width identity is not.

All four final console captures had zero errors and zero warnings. Detector finding messages use console LOG. Injection succeeded; overlays were actually rendered, but were removed/closed during cleanup. Do not tell the user they remain visible.

## Runtime checks and positive evidence

- Office text contrast sample:56 direct-text elements,0 below4.5:1, minimum4.87:1 on dimmed primary-button key hint. Secondary room text#808a98 on#111418 measured5.28:1. Alpha and ancestor opacity were included. SVG illustration text and xterm canvas were excluded; this is not full WCAG certification. See `b-runtime-contrast-office.json`.
- Keyboard: Tab reached New agent with2px visible outline, then More, attention CTA, floor's single roving tab stop. ArrowRight moved from checkout to Refactor auth with visible focus. Shift+F10 opened desk menu, Escape returned to desk; n opened New agent with labeled textarea focused. Code explicitly restores focus on panel/menu close in `src/lib/office.svelte.ts:842`,`:857`,`:870`.
- Reduced-motion emulation: root gained `.still`; b2/b3/b4 counters were unchanged over1050ms. Newly opened panel opacity1 and transform identity; no frozen-invisible panel failure. Preserve this result rather than flagging the global duration rule by itself. xterm cursorBlink:true at `TerminalView.svelte:84` is not linked to reduced-motion in code; actual cursor blinking and real-program motion were not verified. See `b-reduced-motion.json`.
- Desktop narrow support is intentionally shipped: native configuration sets minimum340×420 (`src-tauri/src/lib.rs:309` at baseline). App max760px hides floor behind active panel/terminals; floor container queries provide row/list variants. Single office floor had no initial desktop horizontal overflow. Crowd floor itself measured client330/scroll330 at minimum: overflow was in the attention band, not the whole floor grid.
- Form controls have explicit labels, choices use actual radios, and terminal sets screenReaderMode:true at `TerminalView.svelte:89`. Field edges/focus use meaningful tokens. Status is a word as well as color, with full desk descriptions for AT.
- Performance safeguards: pose beat320ms and stop under document.hidden/reduce (`src/lib/beat.ts:9`,`:31`); terminal imported when Panes instantiated (`Panes.svelte:14`); WebGL fallback/dynamic import (`TerminalView.svelte:123`); resize debounced60ms and same-size guard (`:153`,`:160`); terminals are position-keyed, avoiding tear-down when grid arrangement changes (`Panes.svelte:5`). Existing built dist had main JS173,098 bytes, terminal342,069 and WebGL113,385, loaded separately; no assertion of native CPU/memory performance follows from those sizes.

Recommended order: `/impeccable adapt` for attention band/menu; `/impeccable harden` for focus/footer/error/keyboard behavior; `/impeccable document` for minor vocabulary alignment; finish with `/impeccable polish`. No application source fixes were authorized or made by B.
