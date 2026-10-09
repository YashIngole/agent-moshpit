# Building and working on Agent Moshpit

## What you need

Node 22 (22.12 or later) or Node 24, Rust 1.89 or later, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

## Build and run

```sh
npm install
npm run tauri dev                              # run from source
npm run tauri build                            # make an installer for the system you are on
npm run tauri build -- --debug --no-bundle     # only the program, with debugging on: what the app tests drive
```

The last one leaves `agent-moshpit.exe` in `src-tauri/target/debug`, or in `debug` under your cargo target folder if you have set one. The installers are not signed, so Windows and macOS will warn the first time. The macOS app is signed ad hoc, with no certificate, which is what lets it open at all on Apple Silicon.

A release is made by pushing a tag: `.github/workflows/release.yml` builds the installers for the three systems and attaches them to a draft release, which a person reads and publishes.

Start the app with `--hidden` to go straight to the tray, for example from a start-up entry.

To look at the window without any program behind it, run `npm run dev` and open `http://localhost:1420/?demo=office` in a browser. The scenes are `office`, `calm`, `empty` and `crowd`. Add `&still` to stop anything changing by itself, `&problem` to see a broken `harnesses.json` reported, `&noeditor` for a computer with no editor, and `&open=demo-4` for a click on a notification about that desk.

## How it is put together

```
src-tauri/src/   the core, in Rust
  lib.rs           the window, the tray, notifications, settings, and the commands the window may call
  engine.rs        owns the desks and the terminals, starts programs, looks at them twice a second
  office.rs        the state of every desk, as a pure function of what was seen at it
  status.rs        working, idle or waiting: from Claude Code's own files, or from how a terminal behaves
  harness.rs       the table of programs, harnesses.json, and finding, installing and updating them
  pty.rs           a pseudo-terminal for each program, and the screens kept for a window opened later
  proctree.rs      making sure every program dies with the app (a job object on Windows)
  editor.rs        the editor a file path is opened in
  link.rs          the agent-moshpit:// address that opens a desk
  toast.rs         Windows notifications that can be clicked
  model.rs         what the window is told about the office: plain data
src/             the window, in Svelte 5
  App.svelte       the frame, the office's own keys, dropped files
  components/      the floor, a desk, a person, the amber band, the top bar and its menu,
                   the panes and a terminal, the New agent, Agent programs and Keys panels
  lib/             the bridge to the core, the window's state, the layout of panes, the menus,
                   file paths and links, words, looks, the shared beat, demo data
  styles/          the colours, type and spacing (tokens.css)
tools/e2e/       the tests that drive the window and the real app, and the stand-in programs they use
site/            agentmoshpit.com: one static page. `npm run build:site` builds the window with its demo
                 data into site/demo, which the page shows live; `node tools/site-shots.mjs` takes its pictures
docs/            the guide, the programs, the known limits and this file; the plan for this version,
                 its mockups, the release notes, and two reviews with their screenshots
spikes/          the proof that programs run inside one app with no window of their own
```

The core holds all state. The window is disposable: closing it destroys the webview, and a new one is drawn from a snapshot and from what each terminal last showed. The terminals are drawn with xterm.js. The programs are children of the app, tied to it so that none is left behind.

The look is dark, the office after hours: graphite rooms, matte desks, and the only bright things are screens and the amber that means "needs you". Type is Geist and Geist Mono, bundled with the app. The colours, type and spacing are in `src/styles/tokens.css`. `DESIGN.md` describes the look as built: what each colour means, the type, the layout and every component. `node tools/make-design-json.mjs` makes `.impeccable/design.json` from it, and stops if it and the stylesheet disagree. `PRODUCT.md` says who this is for and why.

## Tests

```sh
cd src-tauri && cargo test     # the core: desks, status, the program table, terminals
npm test                       # the window's layout of panes, file paths, links, words, looks and times
npm run check                  # the window's types
npm run test:ui                # builds the window and drives it in a headless Edge or Chrome, with demo data
```

One test drives the real app on Windows through WebView2's debugging port. Build the debug program first:

```sh
npm run tauri build -- --debug --no-bundle
node tools/e2e/terminals.mjs
```

`npm run test:app` runs the same test. It looks for `agent-moshpit.exe` in `src-tauri/target/debug`, or under `CARGO_TARGET_DIR` when that is set. If yours was built somewhere else, set `MOSHPIT_APP` to its full path. The agents in this test are stand-in programs added through `harnesses.json`, so no Claude Code, no Codex and no model is used and nothing is spent. It opens the app's window on your desktop while it runs, as the office named `e2e`, and reads the clipboard once without changing it.

`node tools/e2e/real-clis.mjs` starts the real Claude Code and Codex with no task and types nothing into them, to check what only the real programs can show: the question about trusting a folder, and Codex staying idle while its pane is resized. A program that is not installed is skipped.

Every test of the real app runs it as its own named office with its own data folder (`MOSHPIT_INSTANCE`, `MOSHPIT_DATA_DIR`), so an office you have open is left alone, and none of them asks npm for versions.

## The site

`site/` is [agentmoshpit.com](https://agentmoshpit.com): one static page, with no build step of its own. Its opening is the app's own window running with the demo data, which `npm run build:site` builds into `site/demo` (not kept in git). The pictures are taken from that same window by `node tools/site-shots.mjs`; run it again when the window's look changes. `node tools/site-shots.mjs --serve` serves the folder on `http://localhost:4175` to look at it.

The download buttons link to `https://github.com/YashIngole/agent-moshpit/releases/latest/download/<file>`. The release workflow names the files without a version, so those addresses always lead to the newest release and the site does not change when one is made.

It is hosted on Cloudflare as static files. With a Cloudflare login (`npx wrangler login`):

```sh
npm run build:site
npx wrangler deploy --config tools/cloudflare/site.jsonc     # agentmoshpit.com
npx wrangler deploy --config tools/cloudflare/www.jsonc      # www.agentmoshpit.com, which only redirects
```
