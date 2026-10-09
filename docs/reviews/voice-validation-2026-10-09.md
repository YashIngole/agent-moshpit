# Local voice validation — 2026-10-09

This implements the owner's accepted local integration from the
[research record](../research/voice-to-text-2026-10.md). Voice is optional and off
by default. Pinned whisper-rs/whisper.cpp runs on the CPU; CPAL captures the default
native microphone. Models download only on explicit action, into app data.

## Verification record

The v0.4.0 merged tip passed on Windows:

- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 81 tests.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- `npm test`: 52 tests; `npm run check`: zero errors/warnings.
- `npm run test:tools`: two release-manifest tests.
- `npm run test:ui`: all 188 demo checks, including voice states and failures.
- Debug Tauri build and `npm run test:app`: fake-agent terminals and the
  persistence/session/project-identity regression suite.
- Production model downloads: both exact sizes and SHA256 digests verified,
  with no partial files offered as models.
- Both Base and Small recognized the recorded WAV and inserted new text into
  the pinned fake terminal without Enter. Closing into the tray cancelled capture;
  normal process exit omitted that desk's saved screen.
- Release Tauri/NSIS build. Release executable desktop smoke and remote platform
  checks are final publication gates; their run results accompany the release.

The voice extension's finish review returned `ship`. The documenter comparison
confirmed reuse of the incumbent palette, type, top bar, panel and spacing;
DESIGN.md and its sidecar were preserved. An initial merge-script encoding error
and a UI run overlapping a build failed checks; both were corrected and the
affected checks rerun successfully. No failed run is counted as passing.

The Windows baseline was built from research commit
`066ee92c87c196f4abb3d135b2066dedd561ee44`: the release executable is 5,964,800
bytes and its NSIS installer is 2,459,330 bytes. The merged release also includes
main's session/reliability changes, so this comparison is total release growth,
not an isolated attribution to voice.

| Artifact / stored file | Baseline bytes | v0.4.0 bytes | Growth bytes |
| --- | ---: | ---: | ---: |
| Windows release executable | 5,964,800 | 7,255,552 | 1,290,752 |
| Local NSIS installer | 2,459,330 | 2,866,447 | 407,117 |
| Bundled dependency notices | — | 16,188 | — |
| Downloaded Base Q5_1 | — | 59,707,625 | — |
| Downloaded Small Q5_1 | — | 190,085,487 | — |

Executable + notices + one verified model total 66,979,365 bytes with Base or
197,357,227 bytes with Small. These are measured-file subtotals, not a measurement
of an installed directory: uninstaller, WebView2/cache, office files and filesystem
allocation are excluded. The owner's installed app was not changed. The earlier
approximately 200 MB / 70 MB descriptions remain estimates of installation use.

The official whisper.cpp v1.8.3 `samples/jfk.wav` fixture is 352,078 bytes, SHA256
`59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.
The debug-only fixture hook requires a named instance and an explicit isolated
data directory. Invalid fixtures fail without falling back to a real microphone.
Release builds do not read the fixture variable.

In the final debug fixture run, Stop-to-result was 17,423 ms for Base and
73,009 ms for Small on this Windows machine. Each is one English clip, with a
release build running in the background; neither is a latency or accuracy
benchmark. RSS was not measured. The exact fixture and both model files remain
only in isolated test storage.

## Limits

No physical microphone or owner's office is used in automated validation. The
tests use fake agents, isolated storage and recorded audio. They do not change
the system clipboard. Accuracy, quantized inference RSS, and general latency are
not established; one fixture timing is not a benchmark. Hindi/Hinglish quality is
not established.

Windows uses target-generated bindings because the pinned crate's packaged
bindings include glibc layout assertions that fail on Windows. Existing CMake,
C++ tools and libclang were reused; no unrelated system software was installed.
Linux uses packaged bindings and ALSA build dependencies. macOS uses generated
bindings, CoreAudio, a microphone usage declaration and an audio-input entitlement.

Physical microphone permission denial, device routing/loss, old-CPU behavior and
native microphone/tray behavior on each platform remain manual checks. A real
desktop microphone smoke test needs the owner's separate explicit permission.
Models are outside the installer. Audio and recognized text stay local; model
requests go to Hugging Face. The app omits voice-used desks from its saved screen
files to avoid persisting terminal echo of a transcript.
