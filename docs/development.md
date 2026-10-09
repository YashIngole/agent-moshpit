# Building and working on Agent Moshpit

## What you need

Node 22 (22.12 or later) or Node 24, Rust 1.89 or later, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

Local voice adds **CMake and a C++ toolchain** for pinned `whisper-rs = 0.16.0` / `whisper-rs-sys = 0.15.0` (vendored whisper.cpp 1.8.3), and native `cpal = 0.15.3`. GPU features and OpenMP are off. `.cargo/config.toml` disables host-native tuning and SSE4.2/AVX/AVX2/FMA/F16C/AVX512/AVX-VNNI options for distributable CPU builds. Do not replace these with `-march=native` or local-only flags. The actual CMake cache is included in the Windows verification record; old-CPU hardware has not been tested.

- **Windows:** Visual Studio C++ tools, CMake, and a discoverable `libclang.dll` (`LIBCLANG_PATH` can name its folder). The pinned crate's packaged bindings include glibc layout assertions that fail on Windows, so bindings are generated for this target. Existing LLVM/libclang can be reused; no audio SDK is installed separately.
- **Linux:** add `libasound2-dev` and `cmake` to Tauri's build packages. `WHISPER_DONT_GENERATE_BINDINGS=1` uses the packaged bindings on 64-bit Linux and avoids libclang. Runtime capture requires working ALSA libraries/default input (PipeWire/PulseAudio systems commonly expose an ALSA route).
- **macOS:** CMake, Xcode command-line tools and libclang for target bindings. Native capture uses CoreAudio. `src-tauri/Info.plist` supplies the microphone usage description and `Entitlements.plist` the audio-input entitlement. The CI/release workflow retains both ARM and Intel Mac targets; permission behavior requires a bundled app smoke test.

CI uses the runner's existing LLVM on Windows/macOS and packaged Linux bindings. The release workflow requires the cross-platform checks before building installers. Model weights are never bundled or fetched during a build. See the [voice validation record](reviews/voice-validation-2026-10-09.md) for actual platform results and outstanding microphone checks.

## Build and run

```sh
npm install
npm run tauri dev                              # run from source
npm run tauri build                            # make an installer for the system you are on
npm run tauri build -- --debug --no-bundle     # only the program, with debugging on: what the app tests drive
```

The last one leaves `agent-moshpit.exe` in `src-tauri/target/debug`, or in `debug` under your cargo target folder if you have set one. The installers are not signed, so Windows and macOS will warn the first time. The macOS app is signed ad hoc, with no certificate, which is what lets it open at all on Apple Silicon.

A release is made by pushing a tag: `.github/workflows/release.yml` builds the installers for the three systems and attaches them to a draft release, which a person reads and publishes. The draft's text is `docs/releases/<tag>.md`.

The same run signs what an installed office updates itself from, with the app's own update key (the repository secret `TAURI_SIGNING_PRIVATE_KEY`; its public half is in `tauri.conf.json`), and writes `latest.json`, which a running office reads to learn that a newer version is out. That signing is asked for by `src-tauri/tauri.release.conf.json`, which only the release run uses, so a build on your own computer needs no key. The key has nothing to do with Windows or Apple code signing.

To change a draft's text after it is made, use `gh release edit <tag> --draft --notes-file docs/releases/<tag>.md`, and look at the draft's tag before publishing: a draft changed without naming its tag was once published under a made-up one (`untagged-…`), which breaks the addresses in `latest.json`.

Before a draft is published, `node tools/e2e/update-fetch.mjs <tag>` tries its signed Windows installer against the key built into the app: the real app, told only to fetch, must accept the real file and refuse it with one byte changed. Nothing is installed.

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
  voice/           bounded device PCM conversion, local CPU recognition, pinned verified model downloads,
                   cancellation generations, and no-Enter terminal delivery
src/             the window, in Svelte 5
  App.svelte       the frame, the office's own keys, dropped files
  components/      the floor, a desk, a person, the amber band, the top bar and its menu,
                   the panes and a terminal, the New agent, Agent programs and Keys panels
  lib/             the bridge to the core, the window's state, the layout of panes, the menus,
                   file paths and links, words, looks, the shared beat, demo data
  styles/          the colours, type and spacing (tokens.css)
tools/e2e/       the tests that drive the window and the real app, and the stand-in programs they use
site/            agentmoshpit.com: the static home page and guides. `npm run build:site` builds the window with its demo
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
npm run test:tools             # repeatable release manifests
npm run check                  # the window's types
npm run test:ui                # builds the window and drives it in a headless Edge or Chrome, with demo data
```

One test drives the real app on Windows through WebView2's debugging port. Build the debug program first:

```sh
npm run tauri build -- --debug --no-bundle
node tools/e2e/terminals.mjs
```

`npm run test:app` runs that test followed by the persistence and project-identity regressions in `tools/e2e/reliability.mjs`. It looks for `agent-moshpit.exe` in `src-tauri/target/debug`, or under `CARGO_TARGET_DIR` when that is set. If yours was built somewhere else, set `MOSHPIT_APP` to its full path. The agents in this test are stand-in programs added through `harnesses.json`, so no Claude Code, no Codex and no model is used and nothing is spent. It opens the app's window on your desktop while it runs, as the office named `e2e`, and reads the clipboard once without changing it.

`node tools/e2e/real-clis.mjs` starts the real Claude Code and Codex with no task and types nothing into them, to check what only the real programs can show: the badge agreeing with the visible trust dialog or ready prompt, two empty Codex desks in the same folder refusing to borrow existing session IDs, and Codex staying idle while its pane is resized. A program that is not installed is skipped.

Every test of the real app runs it as its own named office with its own data folder (`MOSHPIT_INSTANCE`, `MOSHPIT_DATA_DIR`), so an office you have open is left alone, and none of them asks npm for versions.

### Voice checks without a microphone

Unit tests cover actual PCM formats/channel boundaries, 60-second bounds, silence/short rejection, ANSI/control sanitization, state cancellation, pinned file size/hash, atomic completion, hidden panes, restarted PTY generations, and excluding voice echo from saved screens. `npm run test:ui` adds deterministic demo listening/transcribing/failure/download/remove/cancel tests; it stubs clipboard writes and never uses a microphone or downloads a model.

The debug build accepts **`MOSHPIT_VOICE_WAV` only with a named `MOSHPIT_INSTANCE` and explicit `MOSHPIT_DATA_DIR`**. It requires PCM16 WAV, no longer than 60 seconds, and feeds the same conversion/recognition path. A bad fixture fails without opening a microphone. Release builds never read this variable. This is a test hook, not an audio-file feature.

For the opt-in real-app fixture test, provide isolated storage with whisper.cpp v1.8.3's `samples/jfk.wav` and the pinned models, then:

```powershell
$env:CARGO_TARGET_DIR = 'D:/rust/target/voice-to-text'
$env:MOSHPIT_INSTANCE = 'voicebuild'
$env:MOSHPIT_DATA_DIR = 'D:/rust/target/voice-to-text/test-data'
npm run tauri build -- --debug --no-bundle
$env:MOSHPIT_VOICE_FIXTURES = 'D:/rust/target/voice-fixtures'
node tools/e2e/voice-app.mjs             # requires existing model fixtures
node tools/e2e/voice-app.mjs --download  # explicitly exercise production model downloads if absent
```

The helper supplies its own named instance and temporary data folder, starts only the fake agent, verifies both stored model hashes/sizes, checks WAV recognition without Enter, and closes/reopens the window to verify cancellation. It leaves model fixtures in the supplied test storage for reuse; nothing is installed or copied into the user's office. Fixture latency is one recorded clip, not a performance or accent-quality benchmark. Real desktop microphone smoke checks need separate explicit permission. [Results and remaining checks](reviews/voice-validation-2026-10-09.md).

## The site

`site/` is [agentmoshpit.com](https://agentmoshpit.com): the home page and the guides under `site/guides`, all static, with no build step of their own. Its opening is the app's own window running with the demo data, which `npm run build:site` builds into `site/demo` (not kept in git). The pictures are taken from that same window by `node tools/site-shots.mjs`; run it again when the window's look changes. `node tools/site-shots.mjs --serve` serves the folder on `http://localhost:4175` to look at it.

The download buttons link to `https://github.com/YashIngole/agent-moshpit/releases/latest/download/<file>`. The release workflow names the files without a version, so those addresses always lead to the newest release. One line of the site does name a version: `softwareVersion` in the home page's structured data. Change it when a release is published.

For search engines there are `site/robots.txt` and `site/sitemap.xml`; a new page goes into the sitemap by hand, and a changed one gets a new `lastmod`. Each guide says at its top when it was last checked against the programs it describes. When Claude Code, Codex or the app changes what a guide says, check it again and change that date and the two in its structured data. The guides' pictures are the `guide-*.webp` that `node tools/site-shots.mjs --guides` takes.

After building the demo, serve the actual Cloudflare configuration locally with `npx wrangler dev --config tools/cloudflare/site.jsonc --local --port 4177`, then run `node tools/check-site.mjs http://127.0.0.1:4177`. It checks the sitemap, each page's metadata and structured data, internal links and fragments, image assets, headings, reading without JavaScript, indexing headers, the install scripts and a real 404 response. After deploying, run `node tools/check-site.mjs https://agentmoshpit.com` against the live site.

It is hosted on Cloudflare as static files. With a Cloudflare login (`npx wrangler login`):

```sh
npm run build:site
npx wrangler deploy --config tools/cloudflare/site.jsonc     # agentmoshpit.com
npx wrangler deploy --config tools/cloudflare/www.jsonc      # www.agentmoshpit.com, which only redirects
```


## Reliability checks added on 9 October 2026

| Reviewed problem | Fix and regression coverage |
| --- | --- |
| Missing saved folder launched in a different directory | Wake returns a recovery error and preserves the saved cwd; core and Windows desktop checks. |
| Two Codex desks could adopt the same conversation | Own terminal ID plus unambiguous metadata resolution; same-folder collision tests, a real CLI negative check, and a desktop fixture covering own-session restart, session switching and custom CODEX_HOME. Legacy guessed IDs cannot auto-resume. |
| False folder-trust badges | Visible dialog detection, including a ready-prompt negative case; core, stand-in and real CLI checks. |
| Corrupt saved desks were overwritten on quit | Protected recovery state, visible file error, backup of the prior readable version, revision ordering and preserved screens; storage and Windows desktop checks. |
| Late process errors looked asleep | Unexpected nonzero exits become Failed regardless of age; state-machine check. |
| Custom CLI configuration homes ignored | CODEX_HOME and CLAUDE_CONFIG_DIR respected; root-resolution tests. |
| Zoom keyboard navigation did not update watched desks | Both navigation paths report the visible desk; browser assertions on the bridge calls. |
| Checked-out branches remained stale | Periodic refresh for every running adapter, with Claude session cwd when available; live shell/HEAD regression. |
| Quit/update omitted unfinished package jobs | Shared quit guard includes running jobs; engine regression. |
| Equal folder names merged different projects | Canonical project identity, common repository identity for worktrees and disambiguated labels; unit and desktop regressions. |
| Tray New agent could arrive before listeners | Durable pending request, consumed after subscription; bridge race and new-window browser checks. |
| More menu lacked keyboard navigation | Focus entry/return, arrows, Home/End and Escape; browser checks. |

A 30,000-line terminal test checks bounded replay and retained rendering state. The terminal component loads separately from the initial UI bundle. Children inherit the same expanded PATH used to discover programs, including login-shell paths on Unix.

CI runs the browser and core checks on Windows, macOS and Linux, and the real stand-in desktop/persistence flow on Windows after building the packaged debug executable. A release resolves its tag to one commit, runs that CI workflow on the commit, and only then creates a draft and builds installers from the same commit. Signature assets stay on the release so manifest generation can be repeated. Workflow changes require a pushed CI run for remote validation.

`cargo audit --file src-tauri/Cargo.lock` on 9 October found zero vulnerability entries and two informational warnings in Tauri's Linux GTK dependency tree: [GLib 0.18.5 iterator unsoundness](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) and [unmaintained proc-macro-error 1.0.4](https://rustsec.org/advisories/RUSTSEC-2024-0370.html). GLib's published fix is in 0.20+, incompatible with GTK 0.18's dependency requirements; adding another version would leave the affected copy present. These warnings remain visible in CI. Replacing or forking the GTK stack needs Linux validation; it is not claimed fixed by these application changes.


Final local validation: 71 Rust tests, 49 frontend unit tests, 2 release-tool tests, 156 browser checks, Svelte types, Clippy with warnings denied, and actionlint all passed. The packaged Windows app passed the full stand-in terminal flow and the persistence/session regressions. Real Claude Code and Codex startup checks passed without sending a model task. The published v0.3.0 update passed signature verification; a one-byte modification was rejected, with installation disabled. npm audit reported zero vulnerabilities. The packaged debug build is ready; the installed office was not replaced.
