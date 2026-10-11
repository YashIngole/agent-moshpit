# Building and working on Agent Moshpit

## What you need

Node 22 (22.12 or later) or Node 24, Rust 1.89 or later, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

Voice adds **CMake and a C++ toolchain** for the local speech engine, pinned `transcribe-cpp = 0.3.1` (transcribe.cpp with its own ggml, built statically, CPU only), and native `cpal = 0.15.3` for the microphone. `.cargo/config.toml` passes the engine's CMake flags in `TRANSCRIBE_CMAKE_ARGS`: host-native tuning off; SSE4.2, AVX, AVX2, FMA, F16C and BMI2 on; AVX-512 and AVX-VNNI off, so one x86-64 build runs on any processor from 2013 on (Apple Silicon uses NEON, and Metal). Do not replace these with `-march=native` or local-only flags. Cloud voice adds nothing to build: `tungstenite` over the app's own rustls, and `keyring` for the system keychain.

- **Windows:** Visual Studio C++ tools and CMake. No libclang: the engine's bindings come pregenerated. The x64 target links the C and C++ runtimes statically (`+crt-static`, and `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` for the engine); CI inspects DLL imports to reject an unbundled Microsoft C++ runtime dependency. Raw desktop tests also check the hosted runner's WebView2 Runtime.
- **Linux:** add `libasound2-dev`, `libdbus-1-dev` and `cmake` to Tauri's build packages. Capture needs a working ALSA input (PipeWire and PulseAudio systems commonly expose an ALSA route). Keeping a cloud key needs a running Secret Service (GNOME Keyring or KWallet); without one, the panel offers to keep the key for the session only.
- **macOS:** macOS 10.15 or newer, CMake and the Xcode command-line tools. Tauri and direct Cargo builds both set the deployment minimum to 10.15. Native capture uses CoreAudio. `src-tauri/Info.plist` supplies the microphone usage description and `Entitlements.plist` the audio-input entitlement. Keys go to the login keychain. The CI/release workflow retains both ARM and Intel Mac targets; permission behavior requires a bundled app smoke test.

The release workflow requires the cross-platform checks before building installers. Model weights are never bundled or fetched during a build. CI and the release workflow start the speech worker (`agent-moshpit --voice-worker --self-test`) on every system they can run it on. See the [engine decision](research/voice-engines-2026-10.md) for why this engine and model, with the numbers measured.

The engine's build script reruns when `TRANSCRIBE_CMAKE_ARGS` changes, but not for a changed macOS deployment target or compiler. After changing those, run `cargo clean --manifest-path src-tauri/Cargo.toml -p transcribe-cpp-sys` in this checkout's target directory before rebuilding, with `--release` for release artifacts and `--target <triple>` when the build uses an explicit target. On Windows, `CMAKE_POLICY_DEFAULT_CMP0091=NEW` makes CMake honour the static runtime setting; CI refreshes the native package and checks the executable's DLL imports.

## Build and run

```sh
npm install
npm run tauri dev                              # run from source
npm run tauri build                            # make an installer for the system you are on
npm run tauri build -- --debug --no-bundle     # only the program, with debugging on: what the app tests drive
```

The last one leaves `agent-moshpit.exe` in `src-tauri/target/debug`, or in `debug` under your cargo target folder if you have set one. The installers are not signed, so Windows and macOS will warn the first time. The macOS app is signed ad hoc, with no certificate, which is what lets it open at all on Apple Silicon.

A release is made by pushing a tag: `.github/workflows/release.yml` builds the installers for the three systems and attaches them to a draft release, which a person reads and publishes. The draft's text is `docs/releases/<tag>.md`.

Before anything is built, the run checks two things and stops if either fails: the tagged commit is on `main` (merge first, then tag), and `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` all carry the tag's version (`node tools/release-check.mjs <tag>` checks the same locally). `latest.json` offers the tag's version, so a build reporting another one would be offered to every office again and again. The workflows' third-party actions are pinned to full commits, because the release job holds the update-signing key; update a pin by resolving the new tag to its commit.

The last job also writes `SHA256SUMS`, the SHA-256 of every installer under a first line naming the release (`tools/release-checksums.mjs`), signs it with the installer key (`ssh-keygen -Y sign`, namespace `agentmoshpit-install`), checks that signature against the pinned public key in `tools/install-signers`, and attaches both files. `site/install.sh` and `site/install.ps1` pin the same key and refuse a release whose list it did not sign, or an installer whose hash differs; `npm run test:tools` runs both scripts against a stand-in release to prove it. The private half is the repository secret `INSTALL_SIGNING_KEY`. It is separate from the update key and much less precious: if it is lost, make a new pair (`ssh-keygen -t ed25519 -N ""`), put its public line in `tools/install-signers` and both scripts, set the secret, and publish a release before deploying the site, because the deployed scripts check the newest release. For the same reason, deploy install-script changes only after a release that carries a list signed by the key they pin.

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
  voice/           dictation: the microphone, pause detection, settled words typed with no Enter, cancellation;
    local/         the speech worker (`--voice-worker`, a child process) and its framed protocol
    cloud/         the four providers over one WebSocket each, keys in the system keychain
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

`npm run test:launch` checks the launch form with demo catalogs: provider-specific drafts, refresh adding a new model and a new effort level, duplication, reset, custom sandbox controls, narrow windows and failed discovery. The production picker has no baked-in model list.

`cargo test --manifest-path src-tauri/Cargo.toml catalog::tests::installed_clis_return_live_catalogs_without_inference -- --ignored` checks the real installed CLIs without sending a user prompt. After building the desktop app, `node tools/e2e/live-catalogs.mjs` checks real desktop IPC, catalogs and permission choices without creating any agent. Set `MOSHPIT_APP` when the executable is outside the default target directory. The reliability suite also checks launch overrides across agent restart and a full app reload.

```sh
cd src-tauri && cargo test     # the core: desks, status, the program table, terminals
npm test                       # the window's layout of panes, file paths, links, words, looks and times
npm run test:tools             # repeatable release manifests, version checks
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

The Windows debug build accepts `MOSHPIT_TEST_DEBUG_PORT` only for a named office with an explicit data directory. Test helpers pass this port through the WebView2 API because Runtime 150+ ignores environment browser flags in elevated hosts such as GitHub runners. Release builds do not read this test variable.

### Voice checks without a microphone

Unit tests cover PCM formats and bounds, pause detection, joining settled words, ANSI/control sanitization, state cancellation, settings read leniently (and by 0.6), the pinned model's size, hash and check stamp, resumed downloads, the worker's framing, each cloud provider's messages (recorded from their documentation and probes), and that a socket read comes back at once after the handshake. `npm run test:ui` and `node tools/e2e/voice-ux.mjs` (after `npm run build`) drive the demo office: setup for both engines, keys (saved, session-only, unchecked, rejected), word hints, silence, failures (`?voice=auth`, `quota`, `offline`, `worker-crash`, `store-unavailable`, `unsupported` and others), narrow layouts and terminal focus. The demo never uses a microphone, the network or a real key. On Windows, `node tools/e2e/voice-setup-app.mjs` checks the desktop IPC, input listing, saved settings and that 0.6 settings carry over; set `MOSHPIT_APP` when the executable is outside the default debug target folder. CI runs the browser suites on all three systems and the native setup check on Windows. `cargo test --manifest-path src-tauri/Cargo.toml voice:: -- --ignored` runs the two tests that touch this machine: listing its microphones without recording, and keeping a key in the real system keychain.

The debug build accepts **`MOSHPIT_VOICE_WAV` only with a named `MOSHPIT_INSTANCE` and explicit `MOSHPIT_DATA_DIR`**. It requires PCM16 WAV, no longer than the five-minute cap, and feeds the same path a microphone does; a bad fixture fails without opening a microphone. `MOSHPIT_VOICE_WAV_PACED=1` releases it as fast as it was spoken, as a microphone would. Under the same conditions `MOSHPIT_VOICE_ENDPOINT` (say `ws://127.0.0.1:9301`) sends a cloud engine's socket to a test server instead of the provider, with the provider's own path and query; plain `ws://` is accepted only to this computer, and only by a debug build. Release builds read none of these. They are test hooks, not features.

For the opt-in real-app test, put whisper.cpp's `samples/jfk.wav` and the model file (`nemotron-speech-streaming-en-0.6b-Q8_0.gguf`) in a fixture folder, then:

```powershell
$env:CARGO_TARGET_DIR = 'D:/rust/target/voice-engines'
npm run tauri build -- --debug --no-bundle
$env:MOSHPIT_APP = 'D:/rust/target/voice-engines/debug/agent-moshpit.exe'
$env:MOSHPIT_VOICE_FIXTURES = 'D:/rust/target/voice-fixtures'
node tools/e2e/voice-app.mjs             # requires the model fixture
node tools/e2e/voice-app.mjs --download  # explicitly exercise the production download if absent
```

It starts its own named office with the fake agent, checks the model's size and hash, dictates the clip at speaking pace, and checks that words are typed before Stop and that Enter is never pressed. It kills the speech worker mid-sentence and dictates again. Then it runs cloud recordings against `tools/e2e/voice-mock.mjs`, a pretend Deepgram on this computer: words typed while listening, what the provider was asked for (English, Nova-3, no training, the word hints, the key in its header), a rejected key, a lost connection, and a key kept in and removed from Windows Credential Manager. Last, it checks that no key was written to any file in the office's folder. Numbers from one clip are not a benchmark; the [engine decision](research/voice-engines-2026-10.md) has those. Real microphone checks need separate explicit permission.

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
