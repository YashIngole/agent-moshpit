---
name: Agent Moshpit
description: "A small desktop office for coding agents: each one a real CLI in a terminal pane, drawn as a person at a desk, after hours."
colors:
  needs: "#ffb224"
  needs-ink: "#140e00"
  done: "#3ccf91"
  trouble: "#ff5f56"
  trouble-ink: "#ff7b73"
  trouble-wash: "#2a1416"
  work: "#82aaff"
  quiet: "#808a98"
  bg: "#0a0c0f"
  carpet: "#111418"
  grid: "rgba(255, 255, 255, 0.028)"
  wall: "#252b33"
  chrome: "#0d0f12"
  panel: "#13161a"
  inset: "#1b1f25"
  line: "#20252c"
  field-line: "#66707d"
  select: "#2c3a55"
  ink: "#e7eaf0"
  ink-2: "#a0a8b4"
  ink-3: "#808a98"
  button: "#e7eaf0"
  button-ink: "#0a0c0f"
  button-hover: "#ffffff"
  focus: "#e7eaf0"
  chair: "#23272e"
  chair-edge: "#343a43"
  desk: "#2a2f36"
  desk-edge: "#1b1f24"
  desk-leg: "#15181c"
  mat: "#1a1d22"
  kbd: "#0f1114"
  keycaps: "#3a414b"
  bezel: "#0b0d10"
  screen: "#10161f"
  screen-off: "#07090b"
  term: "#0b0d10"
  term-ink: "#d5dae2"
  term-dim: "#6c7684"
  term-line: "#20252c"
  term-select: "#2c3a55"
  ansi-0: "#1b1f25"
  ansi-1: "#ff5f56"
  ansi-2: "#3ccf91"
  ansi-3: "#ffb224"
  ansi-4: "#82aaff"
  ansi-5: "#c099ff"
  ansi-6: "#86e1fc"
  ansi-7: "#c8d0dc"
  ansi-8: "#5c6573"
  ansi-9: "#ff8a82"
  ansi-10: "#6fe0b0"
  ansi-11: "#ffcb6b"
  ansi-12: "#a6c1ff"
  ansi-13: "#d4b8ff"
  ansi-14: "#b4ecfd"
  ansi-15: "#f4f6fa"
typography:
  headline:
    fontFamily: "'Geist Variable', 'Segoe UI', system-ui, -apple-system, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.01em"
  title:
    fontFamily: "'Geist Variable', 'Segoe UI', system-ui, -apple-system, sans-serif"
    fontSize: "1rem"
    fontWeight: 650
    lineHeight: 1.25
  body:
    fontFamily: "'Geist Variable', 'Segoe UI', system-ui, -apple-system, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: "'tnum'"
  small:
    fontFamily: "'Geist Variable', 'Segoe UI', system-ui, -apple-system, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.4
  note:
    fontFamily: "'Geist Variable', 'Segoe UI', system-ui, -apple-system, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.4
  sign:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "0.875rem"
    fontWeight: 600
    lineHeight: 1.3
  button:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "0.8125rem"
    fontWeight: 650
  meta:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
  status:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "11.5px"
    fontWeight: 400
  label:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "0.75rem"
    fontWeight: 600
    letterSpacing: "0.06em"
  terminal:
    fontFamily: "'Geist Mono Variable', ui-monospace, 'Cascadia Mono', Consolas, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.2
rounded:
  r-1: "7px"
  r-2: "10px"
  r-3: "12px"
  pill: "999px"
spacing:
  s-1: "4px"
  s-2: "8px"
  s-3: "12px"
  s-4: "16px"
  s-5: "24px"
components:
  button-primary:
    backgroundColor: "{colors.button}"
    textColor: "{colors.button-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.r-1}"
    padding: "0 12px"
    height: "32px"
  button-primary-hover:
    backgroundColor: "{colors.button-hover}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.r-1}"
    padding: "0 12px"
    height: "32px"
  button-quiet-hover:
    backgroundColor: "{colors.inset}"
  button-danger:
    backgroundColor: "transparent"
    textColor: "{colors.trouble-ink}"
    typography: "{typography.button}"
    rounded: "{rounded.r-1}"
    padding: "0 12px"
    height: "32px"
  button-danger-hover:
    backgroundColor: "{colors.trouble-wash}"
  icon-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.r-1}"
    size: "28px"
  icon-button-hover:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.ink}"
  field:
    backgroundColor: "{colors.inset}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.r-1}"
    padding: "7px 12px"
    height: "34px"
  choice:
    backgroundColor: "{colors.inset}"
    rounded: "{rounded.r-1}"
    padding: "3px"
  choice-option:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    typography: "{typography.small}"
    rounded: "4px"
    padding: "0 12px"
    height: "28px"
  choice-option-on:
    backgroundColor: "{colors.button}"
    textColor: "{colors.button-ink}"
  status-chip:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    typography: "{typography.status}"
  status-chip-needs:
    textColor: "{colors.needs}"
  status-chip-trouble:
    textColor: "{colors.trouble-ink}"
  program-tag:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "5px"
    padding: "0 6px"
  key:
    backgroundColor: "transparent"
    rounded: "4px"
    padding: "0 5px"
  band:
    backgroundColor: "{colors.needs}"
    textColor: "{colors.needs-ink}"
    padding: "10px 14px 10px 18px"
  band-answer:
    backgroundColor: "{colors.needs-ink}"
    textColor: "{colors.needs}"
    rounded: "{rounded.r-1}"
    padding: "0 14px"
    height: "34px"
  band-answer-hover:
    backgroundColor: "#2a1f04"
  desk:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.r-2}"
    padding: "8px 8px 12px"
  desk-hover:
    backgroundColor: "rgba(255, 255, 255, 0.03)"
  desk-open:
    backgroundColor: "rgba(130, 170, 255, 0.07)"
  spare-desk:
    backgroundColor: "transparent"
    textColor: "{colors.ink-3}"
    rounded: "{rounded.r-2}"
    padding: "12px"
    height: "120px"
  spare-desk-hover:
    backgroundColor: "rgba(255, 255, 255, 0.025)"
    textColor: "{colors.ink}"
  room:
    backgroundColor: "{colors.carpet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.r-3}"
    padding: "12px 14px 14px"
  top-bar:
    backgroundColor: "{colors.chrome}"
    textColor: "{colors.ink}"
    height: "52px"
    padding: "0 12px 0 18px"
  pane-header:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    padding: "7px 8px 7px 10px"
  terminal:
    backgroundColor: "{colors.term}"
    textColor: "{colors.term-ink}"
    typography: "{typography.terminal}"
    padding: "8px 0 0 12px"
  side-panel:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    width: "380px"
  menu-sheet:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    rounded: "{rounded.r-2}"
    padding: "8px"
    width: "280px"
  menu-item:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.r-1}"
    padding: "6px 12px"
  menu-item-hover:
    backgroundColor: "{colors.inset}"
  toast:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    typography: "{typography.small}"
    rounded: "{rounded.r-2}"
    padding: "8px 8px 8px 16px"
  problem-strip:
    backgroundColor: "{colors.trouble-wash}"
    textColor: "{colors.trouble-ink}"
    typography: "{typography.small}"
    padding: "8px 12px 8px 24px"
---

# Design System: Agent Moshpit

## Overview

**Creative North Star: "The Raised Hand"**

The window is an office after hours. Each project is a graphite room on a faint grid, each agent is a person at a matte desk, and the only bright things are their screens and the one colour that means "needs you". How a person sits says how their program is doing. Almost all of it is dark and quiet, and it is drawn that way on purpose: when an agent needs an answer an arm goes up and waves, an amber tag with a question mark pops over the desk, the screen turns amber, and a band in the same amber runs across the top of the window with one button on it, the way to their terminal. One raised hand in a dark room cannot be missed.

The office draws people and furniture. It does not draw work. Every agent is a real command-line program in a terminal pane, and what it says, asks and shows is its own screen, in the sixteen colours a program asks for by number. The office's part of a pane ends at its header: a face, a name, a status and a program tag. There is no conversation view, no diff and no Approve button of the office's own, because the program already has them.

Everything the office does draw is flat vector, made in code: a desk is a few rounded rectangles, a person is a head, a torso and two arms. People are told apart by skin, hair, clothes, what is on their head and what is on their desk, all picked from one stable number, so the same agent always looks the same. The look is dark only; there is no day theme. Density is low on the floor and tight beside the terminals, where the floor can shrink to a strip of names. The owner's references are a virtual office like Gather for the floor and BridgeSpace for the grid of terminals.

**Key Characteristics:**
- A room of people, not a table of rows: a status is a pose, a screen, a word and a dot, together.
- One loud colour for one meaning: amber is "needs you".
- Dark only: graphite surfaces a few steps apart, and light that comes from screens.
- Two voices of type: Geist for names and sentences, Geist Mono for the office's signs, status, tags, buttons and keys.
- A terminal belongs to its program. The office gives it a background, a selection colour and sixteen colours, and draws nothing in it.
- Movement is a few fixed poses on a shared beat; nothing moves while the window is hidden.

## Colors

Graphite, and four colours that each mean one thing.

### Primary
- **Amber** (`needs`, with `needs-ink` for anything written on it): "needs you". The band across the top, the "?" tag over a desk, the waiting agent's screen and the light it throws on the desk, the words "needs you" and their dot, the dot beside the count in the top bar, and the dot on the tray icon. Nowhere else in what the office draws.

### Secondary
- **Done green** (`done`): the tick on a finished agent's screen, its light on the desk, and the dot beside "done".
- **Trouble red** (`trouble` for shapes, `trouble-ink` for words, `trouble-wash` behind a notice): a screen that says `err`, the "!" tag and the dot beside "trouble"; the sentence that says what went wrong; the strip under the bar when the office itself has a problem; the words "Remove this desk" in a menu; the box that asks before a busy agent is stopped, restarted or removed, and the button in it.
- **Screen blue** (`work`): the light of a screen. The dot beside "working", the glow under a working agent's monitor, a faint tint and edge on the desk whose terminal is open, and a ring on the pane that is about to take dropped files or trade places with another.

### Tertiary
- **Furniture** (`chair`, `chair-edge`, `desk`, `desk-edge`, `desk-leg`, `mat`, `kbd`, `keycaps`, `bezel`, `screen`, `screen-off`): the chair, the desk and its legs, the mat, the keyboard and the monitor. Matte greys a step or two apart, so a desk reads as a thing without being lit. What stands on a desk (a can, a mug, a lamp, a plant) is drawn in fixed colours of its own, mostly grey, with a dull leaf green for the plant; that is furniture too, not status.
- **People**: skin, hair and clothes come from short lists in `src/lib/look.ts`, not from tokens. Clothes are dark and muted: charcoal, navy, slate blue, plum and one cream.

### Neutral
- **The building** (`bg`): the dark between rooms, and behind everything.
- **Room** (`carpet`, with `grid` for its lines): a room's floor, graphite under a grid of faint lines 28px apart.
- **Wall** (`wall`): the 1px edge of a room, a quiet button, a program tag and an empty desk; the scrollbar thumb.
- **Chrome, panel, inset** (`chrome`, `panel`, `inset`): the top bar; the side panel, a pane's header, menus, the toast and the find bar; the fill of a field and a face tile, and of anything quiet under the pointer.
- **Lines** (`line` for dividers, `field-line` for the edge of anything typed into or chosen in).
- **Ink** (`ink`, `ink-2`, `ink-3`): names and titles; supporting text and status words; quiet text such as times, counts, placeholders and the line under a nameplate.
- **Quiet** (`quiet`, the same value as `ink-3`): the dot beside "idle", the ring beside "away" and "starting", the dots on a screen that is starting up, and the z's over someone who has gone home.
- **Button** (`button`, `button-ink`, `button-hover`, `focus`): the main button is the colour of ink with dark words on it, and turns white under the pointer. The focus ring is ink.
- **Selection** (`select`): selected text, a shade of the room.

### The terminal
- **Glass** (`term`, `term-ink`, `term-dim`, `term-line`, `term-select`): a terminal's background and its default text, the edge between two panes while it is hovered or held, the 1px line that parts panes, and its selection.
- **Sixteen colours** (`ansi-0` to `ansi-15`): the colours a program asks for by number, the night colours an editor uses. Red, green, yellow and blue (`ansi-1` to `ansi-4`) are the same four values as trouble, done, amber and work. Inside a terminal they are the program's, and mean whatever the program means by them.

### Named Rules
**The Four Meanings Rule.** In what the office draws, amber, done green, trouble red and screen blue each mean one thing and are used for nothing else. Nobody is dressed in amber, green or red. Selected text takes a shade of the room (`select`), never amber. The mark for something new in a terminal, and the one for an update being out, is a dot of ink. Which program an agent is, is a word in a tag, never a colour.

**The Word And Shape Rule.** A status is never colour alone. Each has a word (Needs you, Working, Quiet, Done, Idle, Trouble, Starting, Away), a pose, something on the screen, and a 7px dot, which is a hollow ring for Away and Starting. The dot never appears without its word. Quiet uses the neutral prompt pose and means output paused without confirmed completion; it still receives the safeguards for busy programs.

**The Ink Shade Rule.** Red has one shade for shapes and a lighter one for words: `trouble` for a dot, a tag and a screen, `trouble-ink` for anything that is read. On amber, everything is `needs-ink`. Text is held to 4.5:1 and marks and field edges to 3:1 on every surface they sit on; the tightest pairs of tokens are `ink-3` on `inset` at 4.7:1 and `field-line` on `inset` at 3.3:1.

## Typography

**Display Font:** Geist Variable (with Segoe UI, system-ui, -apple-system)
**Body Font:** Geist Variable (the same family)
**Label/Mono Font:** Geist Mono Variable (with ui-monospace, Cascadia Mono, Consolas)

**Character:** An even, plain sans and its mono, both bundled with the app, so nothing is fetched. The office speaks in two voices, and the difference between them does most of the work that size does elsewhere: everything the office writes sits between 10.5 and 20 pixels.

### Hierarchy
- **Headline** (650, 1.25rem, 1.2, -0.01em): the title of a side panel, and the one line of the empty office, "Nobody is in yet." The New agent title is the same size at 750.
- **Title** (650, 1rem, 1.25): the agent's name on the band.
- **Body** (400, 0.875rem, 1.4): sentences and fields. A desk's name is this size at 600, a menu item at 600 to 650.
- **Small** (400, 0.8125rem, 1.4): a toast, a notice, the list of keys. A form label and an option in a choice are this size at 650; the name in a pane's header is 13.5px at 600.
- **Note** (400, 0.75rem, 1.4): the hint under a field in `ink-2`; a note in a panel and the second line of an item in a desk's menu in `ink-3`. The line under a nameplate is the same voice at 12.5px.
- **Sign** (Geist Mono 600, 0.875rem, 1.3): a room's sign, the folder's name with a slash after it. The wordmark is the same voice at 15px.
- **Button** (Geist Mono 650, 0.8125rem): the words on a button. The band's one button is 600.
- **Meta** (Geist Mono 400, 0.75rem): the counts in the top bar, the line under the name on the band, a version number, "demo data".
- **Status** (Geist Mono 400, 11.5px, lowercase): the line under a desk's name: dot, word, how long, program tag. It is 11px in a pane's header, in the list beside the terminals and in a crowd.
- **Label** (Geist Mono 600, 0.75rem, 0.06em, uppercase): the heads of the sections inside a panel (Anywhere, Terminals, Files open in) and "also waiting" on the band.
- **Terminal** (Geist Mono 400, 13px, 1.2): a program's screen. Ctrl with `=`, `-` or the wheel steps it between 9 and 24px, and Ctrl+0 puts it back.

### Named Rules
**The Two Voices Rule.** Geist Mono is the office speaking as a building: signs, counts, status, program tags, buttons, keys, commands, paths and versions. Geist is for what a person named or reads as a sentence: an agent's name, a panel's title, a description, a form label, a menu item.

**The Numbers Hold Still Rule.** Numerals are tabular everywhere, so a running time does not shift the words beside it.

## Layout

A column: the 52px bar; a strip on the trouble wash when the office itself has something wrong to report; the amber band while someone is waiting and the floor is all there is; then the work area, which takes whatever is left. While terminals are open the band is a chip in the bar instead, so a hand going up never moves a terminal under the keyboard. Startup warnings survive desk navigation, and remain available in the menu after explicit dismissal.

With no terminal open, the work area is the floor. Rooms wrap like words, 20px apart, on the dark of the building. Each room asks for the width of its desks, 176px each and 12px between for up to five, so two rooms of three share a row in a 1240px window, and grows to fill its row. Inside a room, desks sit on a grid of columns at least 176px wide and 12px apart, and a desk is never drawn wider than 236px: a room of one that fills a row keeps its carpet. A + on the room's sign seats someone else in that folder; there is no empty desk in a room. Desks keep their place: nothing on the floor is sorted by status. With more than ten agents the floor draws everyone smaller, in columns of 140px, and drops the line about what each is doing.

Opening a desk puts its terminal to the right of the floor. The floor begins at 420px, and is the 264px strip whenever the terminals need the room: two or more panes, more than ten agents, or a window under 1180px, until a width has been chosen. It then keeps that choice, never under 200px. The edge between them can be dragged, moved 24px at a time with the arrow keys, or double-clicked to switch between the strip and 420px. Compact floor in the menu exposes the same choice. Beside terminals the floor's gutters tighten to 12px, and the floor answers to its own width, not the window's. Under 560px a desk becomes a row, the drawing at 116px with the nameplate beside it. Under 340px the drawing is 84px and the line about what they are doing goes. Under 300px the floor is a list, like a terminal's tabs: each desk is a 30px face, a name and a status, and whoever wants a look (needs you, then trouble, then done) is gathered under a "waiting" sign above the rooms.

The terminals share their area as a grid, placed by position: one, two side by side, or rows of several. A click on a desk adds their terminal beside the others, or puts it away again when it is there to be seen; it never takes another's place. An agent who needs you is gone to, not put away, and so is a terminal waiting behind one that has the room. With Ctrl held (Cmd on a Mac) their terminal takes the place of the one in use, so the others keep their size: never a pane whose agent needs you, is in trouble or is being dictated into, when the calmest other pane makes room instead, and a toast names it with Undo. In a window under 760px, where the floor and the terminals are never seen together, it is the other way about: a click goes to their terminal in place of the one showing. Every edge between two panes can be dragged, moved with the arrow keys, or double-clicked to make the panes even. A pane's header can be dragged onto another pane to trade places with it. A pane can be given the whole area; the others are kept behind it and counted on a chip in its header.

The side panel (New agent, Agent programs or Keys, one at a time) is 380px wide, on the right. Beside the floor it takes its width from the floor; when terminals are open it lies over them on a lift instead of squeezing them. In a window narrower than 760px the terminals, or the panel, take the whole work area and the floor waits behind. The New agent actions sit outside the scrolling fields. The band keeps its name and action at every width, wrapping the other waiting names onto their own row. Below 1100px the top bar folds its counts into one button that opens them in the menu; the wordmark gives way below 900px while the chip is there, and below 620px always. The window can be as small as 340 by 420. Menus scroll inside the available height.

Spacing runs on five steps: 4, 8, 12, 16 and 24px. A few values sit off them and recur: the floor's 20px gutter, a room's 14px padding, 18px at the left of the bar and the band, and 6 or 7px between the parts of a status line.

## Elevation & Depth

The rooms are flat. Depth is drawn or tonal, not cast: every desk stands on a soft ellipse of shadow that is part of the drawing, and surfaces are told apart by a step of graphite (the building, a room, a panel, a field). The one soft thing is light: a screen throws a blurred pool of its colour onto the desk, and so does a desk lamp. A box shadow is kept for what floats above the window.

### Shadow Vocabulary
- **Ground** (an ellipse filled with `rgba(0, 0, 0, 0.45)`, drawn under each desk): furniture standing on the floor.
- **Spill** (an ellipse in the screen's colour at 16% opacity, blurred 6px): the light of a screen falling on the desk. Blue at work; amber, green or red by status; grey when idle or starting; none when they have gone home.
- **Lift** (`box-shadow: 0 1px 2px rgba(0, 0, 0, 0.45), 0 12px 32px -10px rgba(0, 0, 0, 0.62)`): a menu, the toast, the find bar, the note shown while files are dragged over a pane, and the side panel when it lies over terminals.

### Named Rules
**The Stand On The Floor Rule.** Things in a room stand on a drawn shadow. A box shadow is for something that floats above the window, and it is always the same one.

**The Light Is Light Rule.** Nothing glows to show that it is selected, pressed or important. The only glow in the window is light from a screen or a lamp. State is shown with a fill, a 1px line or a 2px ring.

## Shapes

Soft, in three steps: 7px for anything pressed or typed into, 10px for a desk, a menu, the toast and the find bar, 12px for a room. Pills (999px) are kept for small named or counted things: "demo data", the names of others waiting, the chip that counts the panes behind, a folder used before. Inside a control, corners drop to 4 or 5px: a key cap, a program tag, an option in a choice.

Everything that needs an edge gets a 1px line: `wall` around a room and around a quiet control, `line` between the parts of the chrome, `field-line` around a field. A dashed 1px line means a place where something can be added. Two-pixel lines mark where something is going: the focus ring (ink, 2px off the edge), the line of ink along the top of the pane the keyboard goes to, the blue ring on a pane about to take dropped files or another pane, and the ring that shows for a moment on a desk found from the top bar.

Drawings are built from rounded rectangles, circles and strokes with round ends, in flat fills. What a person says is a small tag with a tail, not a cartoon bubble. Icons are drawn in a 16-unit box with round-ended strokes of 1.6 to 1.8.

## Components

### Buttons
- **Shape:** gently rounded (7px), at least 32px tall, 12px of padding either side, the label in Geist Mono at 0.8125rem and weight 650.
- **Primary:** the colour of ink with dark words (`button`, `button-ink`). One per place: "+ new agent" in the bar, Start agent in the form, Carry on in a pane whose program has stopped, install or update when there is something to get.
- **Hover / Focus:** the fill turns white (`button-hover`) over 180ms; pressing moves the button down 1px; a disabled one is at 55% opacity. Focus is a 2px outline in ink, 2px off the edge.
- **Quiet:** transparent with a 1px `wall` edge, `inset` under the pointer. For the action beside a primary one: hide terminals, Cancel, Browse, Dismiss, Keep it.
- **Danger:** transparent, words and edge in `trouble-ink`, `trouble-wash` under the pointer. Only inside the box that asks before a busy agent is stopped, restarted or removed.
- **On the band:** one button, `needs-ink` with amber words at weight 600, 34px tall (30px in a narrow window). Its focus ring is `needs-ink`.
- **Icon buttons:** a 28px square with 7px corners in a pane's header, `ink-2` turning to `ink` on `inset` under the pointer. The panel's close is 30px, the bar's menu button and microphone 32px, the + on a room's sign 24px with a `wall` edge. A row in the strip has a × at its end, like a terminal's tab, shown while the pointer or the keyboard is on it and `trouble-ink` under the pointer; a desk on the floor has none, and is taken away from its menu, with Delete or with a middle click.
- **A key beside its action:** in a menu, the key is written at the end of the item's line in Geist Mono at 0.75rem in `ink-3`, as menus everywhere write it. Buttons in the bar carry their keys in their tooltips, not on their faces. The `ctrl ↵` beside Start agent is the one key cap at rest: `ink-3` at 0.75rem with a 1px edge.
- **Case:** the words are lowercase in the bar, on the band and in Agent programs, and in sentence case in the New agent form, in menus and in notices. This has not been made one way.

### Chips
- **Status:** a 7px dot and its word, lowercase, in Geist Mono. The word is `ink-2`, except "needs you" in amber at weight 600 and "trouble" in `trouble-ink`. Idle is a grey dot; away and starting are a hollow ring.
- **State:** the dot beside "working" dims to 45% on every other beat, and holds still when the window does.
- **Program tag:** which program this person is, as a word in a 1px `wall` edge with 5px corners, Geist Mono at 11px on the usual floor and `t-xs` in the list and crowd, lowercase: claude, codex, antigravity, hermes. It gives way before the status does when there is no room.
- **Pills:** "demo data" in the bar, a folder used before in the New agent form, "+2" in the header of a pane that has the room, and the names of others waiting on the band.
- **News:** a 7px dot of ink before a desk's name when its terminal has printed something since it was last looked at and they are not at work or waiting (someone at work prints all the time); a 6px one on the bar's menu button when an update is out.

### Cards / Containers
- **Room:** graphite under a 28px grid of faint lines, a 1px `wall` edge, 12px corners. Its sign is top left: the folder's name in Geist Mono with a slash in `ink-3`; the + that seats someone else there is at its right. The desks say how many there are.
- **Desk:** no card. A transparent button with 10px corners holding the drawing and a nameplate: the name (two lines rather than cut short), the status line (dot and word, how long, and the program tag when more than one program is at work; the tag gives way before the time), and up to two lines in `ink-3` only when there is something the status does not say: what the agent reported it is doing, what the program asked, or what went wrong (`trouble-ink`). A desk never says its status twice. The pointer lays a 3% white tint over it. The desk whose terminal is open has a 7% blue tint and a 1px blue edge at 24%.
- **Empty desk:** only in the empty office. A dashed 1px `wall` outline with 10px corners, at least 120px tall, a plus in a 34px box and "new agent" in lowercase mono, all in `ink-3`. Under the pointer the outline takes `field-line` and the words take `ink`.
- **Side panel:** flat, the `panel` colour, a 1px line on its left. A header with the title and a close, over a body that scrolls. It arrives from 24px to the right over 240ms. In the New agent form the Start and Cancel buttons stay at the foot however long the form is. The form asks in the order a person thinks: who, what, where (the folder filled in with the terminal in use, else where the last agent started, else the busiest room), then a folded More options line saying what the name, model and permissions are set to.
- **Menu:** one small sheet for a desk or a terminal, opened where it was asked for and kept inside the window: 280px wide, `panel` with a 1px line, 10px corners and the lift. An item is one line: a name at weight 600, and its key at the end. A quiet hint under it, wrapping in full, only where the item ends or keeps something: restart, stop, carry on, remove. It takes `inset` under the pointer or the keyboard, with a 2px inset focus outline for keyboard use. Dividers separate conversation actions, folder actions and removal. Removing a desk is in `trouble-ink`. When an agent is busy, including Quiet, stopping, restarting or removing it first turns the sheet into a box on the trouble wash with the question, a danger button and Keep it. Resume labels and hints name the saved desk session, folder's latest conversation, program's latest conversation, or fresh start. Renaming a desk happens in the same sheet. A terminal's own menu holds the terminal's actions and ends in "More for (name)…", which opens the desk's. The menu at the end of the top bar is the same thing at 268px, in groups parted by a line; a setting there is a tick box at the end of its line.
- **Toast:** one line for a moment, on `panel` with a 1px line, 10px corners and the lift, with one action when there is something to undo. It sits at the foot of the floor, never over the foot of a terminal: beside terminals it keeps to the floor's column, and in a narrow window it moves to the top right.
- **Problem strip:** a line on `trouble-wash` in `trouble-ink` under the bar, with quiet buttons at its end.

### Inputs / Fields
- **Style:** `inset` fill, a 1px `field-line` edge, 7px corners, at least 34px tall. A placeholder is `ink-3`.
- **Focus:** the edge takes the ink colour with the 2px outline tight against it. The pointer lightens the edge to `ink-2`.
- **Choice:** one control holding a few options, for which program takes the task and which editor opens files: the field's own fill and edge with 3px of padding, each option 28px tall with 4px corners in `ink-2` at weight 650, the chosen one filled like a primary button. A program that is not installed yet is `ink-3` with a small "get" beside it.
- **Tick box:** the system's own at 16px, ticked in the button colour.
- **Error:** a line of `trouble-ink` under the form that names the problem and what to do.
- **A command shown:** what an install or an update will run, in mono on the terminal's own background with a 1px line around it.
- **Find bar:** a small bar over the top right of a terminal, on `panel` with a 1px line, 10px corners and the lift: a 200px field 28px tall, a hint in Geist Mono at 11px that turns to "not found" in `trouble-ink`, and a close.

### Navigation
- **Top bar:** `chrome` with a 1px line under it. From the left: the 24px mark and the wordmark "agent moshpit" in Geist Mono at 15px and weight 600; while terminals are open, the amber chip for whoever needs you; then counts of only what wants a look, each a dot and a few words ("1 done", "1 in trouble") in `ink-2`, and nothing when nothing does. From the right: the menu button, the microphone, "+ new agent", and, once a terminal has been opened, the quiet button that hides the terminals and brings them back ("terminals" with how many).
- **The counts:** each is also a way to the next desk in that state, which is ringed in ink for a moment. Under 1100px they fold into one button that lists them.
- **The chip:** amber, 32px tall, 7px corners: a 26px face, their name, "needs you" and how long, and "+1" when others wait. A click, or Ctrl+Shift+A from anywhere including a terminal, opens whoever has waited longest beside the terminals, and the next one when asked again.
- **The floor:** it is one stop for the Tab key, and the arrow keys walk from desk to desk like a grid of icons. Enter opens a desk's terminal.

### The band
The amber strip stays while someone is waiting and the floor is all there is; once terminals are open, the chip in the bar takes its place, and the hand stays up on the desk and the pane's header until the question is answered. From the left: their face on a 44px tile, their name above "needs you", how long, the program and where, then what the program asked when it said (never "is waiting for your answer", which the amber already says), and one button, "open their terminal". It asks nothing and answers nothing itself; the question is on their screen. It drops in from 14px above over 240ms. When several are waiting it shows whoever has waited longest, and lists the others along its lower edge as pills; a pill opens that terminal. At the minimum width the face gives way so the identity and action remain readable.

### A pane
- **Header:** `panel` with a 1px line under it and a 2px line along its top, which is ink on the pane the keyboard goes to and clear on the rest. A 30px face, the name at 13.5px, a status line in Geist Mono at 11px (dot and word, how long, then what the agent reported it is doing, else project and branch), the program tag when more than one program is at work, then the icon buttons: more, give it the room, put it away. A narrow header drops the place and the tag first, then the time.
- **Screen:** the program's own, drawn by xterm.js on `term` in Geist Mono at 13px with a line height of 1.2, 8px in from the top and 12px from the left. Panes are parted by a 1px `term-line`.
- **Edges:** an edge between two panes is 9px to the hand and 3px to the eye, clear until the pointer is on it.
- **When the program has stopped:** a bar at the foot of the pane on `panel`, saying so, with a primary button to carry on or start again. When it stopped because its program is signed out, the bar says that instead ("Claude Code is signed out.") and the button is Sign in.
- **Moving and dropping:** a pane being dragged by its header is at 55% opacity, and one blue ring shows where it will land: a side of another pane, the whole of a pane it would trade places with, or a whole side of the room. Nothing moves or changes size until it is dropped, so no program redraws on the way. A pane with files held over it is ringed the same way.
- **Arranging:** with two or more terminals open the bar has a quiet "arrange" button (an icon alone under 1100px) that opens a menu sheet: five shapes, each with a small drawing of it in `ink-2`, then "Make them even" and the four ways to move this terminal with their keys. A toast names the shape and offers Undo. Under 760px the same items are in the bar's own menu.
- **An install or an update** runs in a pane like any other, with a download mark where the face would be.

### Voice input

The top bar's microphone is always present: one control in two parts inside a 1px `line` edge, a 32px icon button and an 18px arrow. The icon button opens setup when voice is disabled, its model is missing, or no running terminal is selected; otherwise it starts a recording, and it does not take the keyboard from the terminal. During capture its icon is a filled stop square and the control's edge and icon are `work`; while preparing or transcribing it is disabled. It carries no words in any state, so nothing beside it in the bar moves. The arrow opens a sheet like the bar's own menu: a head in the Label voice, "Microphone", one line for each input with a tick of ink at the end of the one in use (System default names the device it means on a second line in `ink-2`); then "Speech engine" the same way, only when more than one engine is ready; then "Voice settings…". Choosing another microphone or engine while recording starts the recording again where it was. Under 520px the arrow gives way, and the choices are in the panel alone.

The side panel uses the existing fields and quiet buttons, and says each thing once: the tick box that turns voice on, readiness in a sentence on `inset`, then the speech engine (this computer, or cloud with your own key, each with one line of what it means), then the engine's own section, then the microphone with a 30px icon button that looks again for devices, and the shortcut with one line on holding and tapping it. What is worth knowing once (what is kept, what ends a recording, the model's memory) is in a closed "Privacy and limits" disclosure at the foot, a list in `ink-2`. On this computer the two model choices follow, the recommended one marked in its second line; model names omit their quantization format. In the cloud, a provider select, then a consent note on `inset` with a 2px `work` rule on its left saying where the audio goes and who bills for it, two plain links (the key page, the prices), the key's state in one line ("Key added · ends in 1234 · in the system keychain") with Test, Replace and Remove, or a password field and Save, and word hints as pill chips with a remove button each. Key errors use `trouble-ink`; the way out they offer (keep it for this session, save without checking) is a quiet button under them. Never amber.

A recording says so in the terminal its words go into: a bar over the top right corner of that pane's screen, as the find bar is (and under it when both are open), 340px wide on `panel` with a 1px line, 10px corners and the lift, its edge `work` while it listens. One row: a 44px input meter (a `work` fill on `wall`, pill corners, the microphone's name in its tooltip), "Listening" in `work` at weight 650, the elapsed time in Geist Mono in `ink-2`, a primary Stop 28px tall and a 28px cancel. A cloud recording names its provider after "Listening", in `ink-2` at normal weight. Under the row, only when there is something to say: the words still being decided in `ink` at `t-md`, the newest kept in sight; or, in `ink-2`, that the engine is still loading, that phrases are typed after each pause, or that the named microphone hears nothing, with "Change microphone" as an underlined link that opens the arrow's menu. It does not name the terminal to the eye, because it is in it; the status says the name aloud. Stop and cancel do not take the keyboard from the terminal. When that terminal is not in front (a panel lies over it, another pane has the room, the floor is all there is, or its pane was put away) the same bar floats over the middle of the work that nothing covers, 420px wide, and gains a line with an arrow and the agent's name, which is the way back to their terminal. While the engine loads or connects, the row is a small spinner and what it is doing. Errors use `trouble-wash` and `trouble-ink`: the sentence takes the row to itself, and Voice setup, Try again and dismissal go under it. Once the words are typed nothing is left over the terminal: that they went in is said aloud and seen where they went; a note that asks for a second look is shown, and so is one for a terminal that is not in front. The meter has accessible numeric values; changing timers stay outside the live announcement.

In the New agent form the task has a 28px microphone at the end of its label. While it listens, one line under the field does what the bar does in a terminal: the status in blue, the elapsed time, the words not yet settled in `ink-2`, then Stop and a cancel. Settled words go into the field at the caret, as typing would.

### Agent messages
A side panel like the others, in three parts each under a 1px line. First, how Claude Code is told: three tick boxes, each a name at weight 650 over a line of `t-sm` in `ink-2` saying what it costs; the one that depends on another is `ink-2` while that one is off, and the preview carries its words "research preview" as a mono tag in a 1px line. Then each agent: the name at weight 600, the program and how many unread as a mono tag at its end, and one sentence in `ink-2` on how it is reached. Then what was sent: who to whom, the kind as a tag, up to three lines of the text, and one sentence on how far it got. That sentence turns from `ink-2` to `ink` once the item was read or acknowledged. It is never one of the four colours: arriving is not a status of an agent.

### The person
One drawing with eight poses. At work they type, read with a hand on the mouse, or think with a hand to the chin, changing now and then. Needing you is a raised hand that waves. Done is leaning back with hands behind the head; trouble is hands on the head; away is asleep on folded arms; idle, quiet and starting sit still with hands on the keys. The terminal cursor also follows the reduced-motion preference.

The monitor says the same thing: lines of code that scroll, with a cursor that blinks; `[y/n]` on amber; `err` on red; a green tick; three dots taking turns while starting; a dim prompt when idle; a dark screen when away. The code on a working screen is in an editor's night colours, which are code and not status. Over the head there is a "?" on amber, a "!" on red or a "…" on grey, and z's over someone who has gone home, who is also drawn duller than the rest. The screen's light falls on the desk and tints the lenses of anyone in glasses.

The face carries the mood in its brows. Eyes have whites so they read on every skin, lids are half shut when busy and the eyes are wide when they need you or something is wrong, and the lines of a face are darker on the two darkest skins. Six skin tones, nine hair colours, seven hair styles, three tops in nine colours, headphones, a beanie or a cap, two kinds of glasses, a beard for some, one thing on the desk and sometimes a second screen are kept in `src/lib/look.ts`.

Loops are poses switched on a shared beat every 320ms, and a blink of 130ms every 4.3 seconds; no loop is a running CSS animation. A new agent walks to their chair once, over 720ms. For the band, a pane's header and the list, the same drawing is cropped to head and shoulders.

### The mark
Someone in a hoodie at a lit screen, on a dark tile, 24px in the top bar. The app icon (`assets/brand/icon.svg`) is the same drawing, sixteen times the size. The tray icon and the favicon (`assets/brand/tray.svg`, `public/favicon.svg`) are that figure fitted to a 16-pixel grid, so the tile's edge, the desk and the two blue lines on the screen each fall on whole pixels. While someone needs you the tray icon gains a plain amber dot at its top right, ringed in the tile's colour so it holds on a light taskbar; its words are in the tray's tooltip.

## Do's and Don'ts

### Do:
- **Do** keep amber for "needs you" alone: the band and the chip in the bar, the "?" tag, the waiting screen and its light, the words and their dot, the dot on the tray icon.
- **Do** give every status its word, its dot, its pose and its screen.
- **Do** use `trouble-ink` for red words and `trouble` for red shapes, and check new text at 4.5:1 on `inset`, the lightest surface it can sit on.
- **Do** set the office's own fixtures (signs, counts, status, tags, buttons, keys, commands) in Geist Mono, and names and sentences in Geist.
- **Do** draw furniture and people from rounded rectangles, circles and round-ended strokes in flat fills, standing on the drawn ground shadow.
- **Do** make loops out of two to four fixed poses on the shared beat, and stop them while the window is hidden or the system asks for less motion.
- **Do** keep desks where they are on the floor. A change of status changes the person, not the seating; only the list beside the terminals gathers whoever is waiting at its top.
- **Do** lead to the terminal. The band, the chip, a desk and a notification all end at the program's own screen.
- **Do** say a thing once. A desk's line, the band and a menu hint each carry only what the status word, the pose and the label do not.
- **Do** say which program an agent is with a word in a tag.
- **Do** put a menu, a toast or a bar that floats on `panel` with a 1px line, 10px corners and the one lift.

### Don't:
- **Don't** draw again what a program already shows: no conversation, no diff, no question and no answer button of the office's own.
- **Don't** dress anyone in amber, green or red, or use a status colour for decoration, for news or for telling programs apart.
- **Don't** sort or filter the floor by status.
- **Don't** use a running CSS animation for a loop; one restyles the page sixty times a second for as long as it exists.
- **Don't** add box shadows to things that stand in a room, or make anything glow that is not light from a screen or a lamp.
- **Don't** imitate wood, fabric or glass with gradients. The only gradients are the lines of a room's grid.
- **Don't** style what is inside a terminal beyond its background, its selection and its sixteen colours.
- **Don't** put a toast over the foot of a terminal, where a program shows its prompt and its choices.
- **Don't** use a dialog where the side panel or a menu will do; asking before a busy agent is ended happens inside the menu that offered it.
