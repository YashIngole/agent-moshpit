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
- **Amber** (`needs`, with `needs-ink` for anything written on it): "needs you". The band across the top, the "?" tag over a desk, the waiting agent's screen and the light it throws on the desk, the words "needs you" and their dot, and the dot beside the count in the top bar. Nowhere else in what the office draws.

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

**The Word And Shape Rule.** A status is never colour alone. Each has a word (Needs you, Working, Done, Idle, Trouble, Starting, Away), a pose, something on the screen, and a 7px dot, which is a hollow ring for Away and Starting. The dot never appears without its word.

**The Ink Shade Rule.** Red has one shade for shapes and a lighter one for words: `trouble` for a dot, a tag and a screen, `trouble-ink` for anything that is read. On amber, everything is `needs-ink`. Text is held to 4.5:1 and marks and field edges to 3:1 on every surface they sit on; the tightest pairs of tokens are `ink-3` on `inset` at 4.7:1 and `field-line` on `inset` at 3.3:1. Two small labels fall short as built, because they are also dimmed with opacity: the `ctrl ↵` hint at the foot of the New agent form (2.7:1) and the "get" beside a program that is not installed yet (3.5:1).

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
- **Meta** (Geist Mono 400, 0.75rem): the counts in the top bar, the head count on a sign, a version number, "demo data".
- **Status** (Geist Mono 400, 11.5px, lowercase): the line under a desk's name: dot, word, how long, program tag. It is 11px in a pane's header, in the list beside the terminals and in a crowd.
- **Label** (Geist Mono 600, 0.75rem, 0.06em, uppercase): the heads of the sections inside a panel (Anywhere, Terminals, Files open in) and "also waiting" on the band.
- **Terminal** (Geist Mono 400, 13px, 1.2): a program's screen. Ctrl with `=`, `-` or the wheel steps it between 9 and 24px, and Ctrl+0 puts it back.

### Named Rules
**The Two Voices Rule.** Geist Mono is the office speaking as a building: signs, counts, status, program tags, buttons, keys, commands, paths and versions. Geist is for what a person named or reads as a sentence: an agent's name, a panel's title, a description, a form label, a menu item.

**The Numbers Hold Still Rule.** Numerals are tabular everywhere, so a running time does not shift the words beside it.

## Layout

A column: the 52px bar; a strip on the trouble wash when the office itself has something wrong to report; the amber band while someone is waiting whose terminal is not already in front of you; then the work area, which takes whatever is left.

With no terminal open, the work area is the floor. Rooms wrap like words, 20px apart, on the dark of the building. Each room asks for the width of its desks, 204px each for up to five, and grows to fill its row. Inside a room, desks sit on a grid of columns at least 196px wide and 12px apart, with an empty desk at the end for adding someone. Desks keep their place: nothing on the floor is sorted by status. With more than ten agents the floor draws everyone smaller, in columns of 140px, drops the line about what each is doing, and swaps the empty desks for a + on each room's sign.

Opening a desk puts its terminal to the right of the floor. The floor keeps the width it was dragged to (420px to begin with, never under 200px) and the terminals take the rest, never less than 420px. The edge between them can be dragged, moved 24px at a time with the arrow keys, or double-clicked to bring the floor down to a 264px strip and back. Beside terminals the floor's gutters tighten to 12px, and the floor answers to its own width, not the window's. Under 560px a desk becomes a row, the drawing at 116px with the nameplate beside it. Under 340px the drawing is 84px and the line about what they are doing goes. Under 300px the floor is a list, like a terminal's tabs: each desk is a 30px face, a name and a status, and whoever wants a look (needs you, then trouble, then done) is gathered under a "waiting" sign above the rooms.

The terminals share their area as a grid, placed by position: one, two side by side, or rows of several. Every edge between two panes can be dragged, moved with the arrow keys, or double-clicked to make the panes even. A pane's header can be dragged onto another pane to trade places with it. A pane can be given the whole area; the others are kept behind it and counted on a chip in its header.

The side panel (New agent, Agent programs or Keys, one at a time) is 380px wide, on the right. Beside the floor it takes its width from the floor; when terminals are open it lies over them on a lift instead of squeezing them. In a window narrower than 760px the terminals, or the panel, take the whole work area and the floor waits behind. The band becomes one line under 720px. The top bar gives things up as it narrows: the key hints in its buttons under 860px; the wordmark, the counts and the "demo data" pill under 620px; the words on its buttons under 520px. The window can be as small as 340 by 420.

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
- **Quiet:** transparent with a 1px `wall` edge, `inset` under the pointer. For the action beside a primary one: back to the floor, Cancel, Browse, Dismiss, Keep it.
- **Danger:** transparent, words and edge in `trouble-ink`, `trouble-wash` under the pointer. Only inside the box that asks before a busy agent is stopped, restarted or removed.
- **On the band:** one button, `needs-ink` with amber words at weight 600, 34px tall (30px in a narrow window). Its focus ring is `needs-ink`.
- **Icon buttons:** a 28px square with 7px corners in a pane's header, `ink-2` turning to `ink` on `inset` under the pointer. The panel's close is 30px, the bar's menu button 32px, the + on a room's sign 22px. The × in the corner of a desk is 24px on `panel` with a `wall` edge; it shows only while the pointer or the keyboard is on that desk, and turns `trouble-ink` under the pointer.
- **A key beside its action:** a key cap in Geist Mono at 0.6875rem with a 1px edge in the colour of the words around it, 4px corners, at 60% opacity: `n`, `ctrl` and a backtick.
- **Case:** the words are lowercase in the bar, on the band and in Agent programs, and in sentence case in the New agent form, in menus and in notices. This has not been made one way.

### Chips
- **Status:** a 7px dot and its word, lowercase, in Geist Mono. The word is `ink-2`, except "needs you" in amber at weight 600 and "trouble" in `trouble-ink`. Idle is a grey dot; away and starting are a hollow ring.
- **State:** the dot beside "working" dims to 45% on every other beat, and holds still when the window does.
- **Program tag:** which program this person is, as a word in a 1px `wall` edge with 5px corners, Geist Mono at 11px, lowercase: claude, codex, gemini, hermes. It gives way before the status does when there is no room.
- **Pills:** "demo data" in the bar, a folder used before in the New agent form, "+2" in the header of a pane that has the room, and the names of others waiting on the band.
- **News:** a 7px dot of ink before a desk's name when its terminal has printed something since it was last looked at; a 6px one on the bar's menu button when an update is out.

### Cards / Containers
- **Room:** graphite under a 28px grid of faint lines, a 1px `wall` edge, 12px corners. Its sign is top left: the folder's name in Geist Mono with a slash in `ink-3`, then the head count.
- **Desk:** no card. A transparent button with 10px corners holding the drawing and a nameplate of three parts: the name, the status line, and up to two lines about what is going on in `ink-3` (`trouble-ink` when something went wrong). The pointer lays a 3% white tint over it. The desk whose terminal is open has a 7% blue tint and a 1px blue edge at 24%.
- **Empty desk:** a dashed 1px `wall` outline with 10px corners, at least 120px tall, a plus in a 34px box and "add an agent" in lowercase mono, all in `ink-3`. Under the pointer the outline takes `field-line` and the words take `ink`.
- **Side panel:** flat, the `panel` colour, a 1px line on its left. A header with the title and a close, over a body that scrolls. It arrives from 24px to the right over 240ms. In the New agent form the Start and Cancel buttons stay at the foot however long the form is.
- **Menu:** one small sheet for a desk or a terminal, opened where it was asked for and kept inside the window: 280px wide, `panel` with a 1px line, 10px corners and the lift. An item is a name at weight 600 with a second, quiet line under it, and takes `inset` under the pointer or the keyboard. Removing a desk is in `trouble-ink`. When an agent is busy, stopping, restarting or removing it first turns the sheet into a box on the trouble wash with the question, a danger button and Keep it. Renaming a desk happens in the same sheet. The menu at the end of the top bar is the same thing at 268px.
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
- **Top bar:** `chrome` with a 1px line under it. From the left: the 24px mark and the wordmark "agent moshpit" in Geist Mono at 15px and weight 600; the state of the office in one line of counts, each a dot and a few words ("1 needs you", "2 working") in `ink-2`, or "all quiet". From the right: the menu button, "+ new agent" with its key, and, once a terminal has been opened, the quiet button that goes between the floor and the terminals.
- **The counts:** each is also a way to the next desk in that state, which is ringed in ink for a moment.
- **The floor:** it is one stop for the Tab key, and the arrow keys walk from desk to desk like a grid of icons. Enter opens a desk's terminal.

### The band
The amber strip that exists only while someone is waiting whose terminal is not already in front of you. From the left: their face on a 44px tile, "needs you" and how long above their name, a sentence saying which program is waiting and where, and one button, "open their terminal". It asks nothing and answers nothing itself; the question is on their screen. It drops in from 14px above over 240ms. When several are waiting it shows whoever has waited longest, and lists the others along its lower edge as pills.

### A pane
- **Header:** `panel` with a 1px line under it and a 2px line along its top, which is ink on the pane the keyboard goes to and clear on the rest. A 30px face, the name at 13.5px, a status line in Geist Mono at 11px (dot and word, how long, project and branch), the program tag, then the icon buttons: more, give it the room, put it away. A narrow header drops the place and the tag first, then the time.
- **Screen:** the program's own, drawn by xterm.js on `term` in Geist Mono at 13px with a line height of 1.2, 8px in from the top and 12px from the left. Panes are parted by a 1px `term-line`.
- **Edges:** an edge between two panes is 9px to the hand and 3px to the eye, clear until the pointer is on it.
- **When the program has stopped:** a bar at the foot of the pane on `panel`, saying so, with a primary button to carry on or start again.
- **Moving and dropping:** a pane being dragged is at 55% opacity, and the one it would trade places with is ringed in blue. So is a pane with files held over it.
- **An install or an update** runs in a pane like any other, with a download mark where the face would be.

### The person
One drawing with eight poses. At work they type, read with a hand on the mouse, or think with a hand to the chin, changing now and then. Needing you is a raised hand that waves. Done is leaning back with hands behind the head; trouble is hands on the head; away is asleep on folded arms; idle and starting sit still with hands on the keys.

The monitor says the same thing: lines of code that scroll, with a cursor that blinks; `[y/n]` on amber; `err` on red; a green tick; three dots taking turns while starting; a dim prompt when idle; a dark screen when away. The code on a working screen is in an editor's night colours, which are code and not status. Over the head there is a "?" on amber, a "!" on red or a "…" on grey, and z's over someone who has gone home, who is also drawn duller than the rest. The screen's light falls on the desk and tints the lenses of anyone in glasses.

The face carries the mood in its brows. Eyes have whites so they read on every skin, lids are half shut when busy and the eyes are wide when they need you or something is wrong, and the lines of a face are darker on the two darkest skins. Six skin tones, nine hair colours, seven hair styles, three tops in nine colours, headphones, a beanie or a cap, two kinds of glasses, a beard for some, one thing on the desk and sometimes a second screen are kept in `src/lib/look.ts`.

Loops are poses switched on a shared beat every 320ms, and a blink of 130ms every 4.3 seconds; no loop is a running CSS animation. A new agent walks to their chair once, over 720ms. For the band, a pane's header and the list, the same drawing is cropped to head and shoulders.

### The mark
Someone in a hoodie at a lit screen, on a dark tile, 24px in the top bar. The app icon, the tray icons and the favicon in `assets/brand/` and `public/` have not been redrawn for this look: they are still the first version's teal tile and oak desk, and the dot the tray icon gains when someone needs you is that version's yellow, not this amber.

## Do's and Don'ts

### Do:
- **Do** keep amber for "needs you" alone: the band, the "?" tag, the waiting screen and its light, the words and their dot, the dot beside the count in the bar.
- **Do** give every status its word, its dot, its pose and its screen.
- **Do** use `trouble-ink` for red words and `trouble` for red shapes, and check new text at 4.5:1 on `inset`, the lightest surface it can sit on.
- **Do** set the office's own fixtures (signs, counts, status, tags, buttons, keys, commands) in Geist Mono, and names and sentences in Geist.
- **Do** draw furniture and people from rounded rectangles, circles and round-ended strokes in flat fills, standing on the drawn ground shadow.
- **Do** make loops out of two to four fixed poses on the shared beat, and stop them while the window is hidden or the system asks for less motion.
- **Do** keep desks where they are on the floor. A change of status changes the person, not the seating; only the list beside the terminals gathers whoever is waiting at its top.
- **Do** lead to the terminal. The band, a desk and a notification all end at the program's own screen.
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
