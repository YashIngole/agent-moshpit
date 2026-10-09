# Local voice validation — 2026-10-09

This implements the owner's accepted local integration from the
[research record](../research/voice-to-text-2026-10.md). Voice is optional and off
by default. Pinned whisper-rs/whisper.cpp runs on the CPU; CPAL captures the default
native microphone. Models download only on explicit action, into app data.

## Verification record

The combined v0.5.0 app source (voice, launch settings/catalogs, internal MCP and
session reliability) passed locally on Windows:

- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 103 tests passed;
  one opt-in installed-CLI catalog check ignored. No live agent task was sent.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- `npm test`: 52 tests; `npm run check`: zero errors/warnings.
- `npm run test:tools`: two release-manifest tests.
- `npm run test:ui`: all 188 demo checks, including voice states and failures.
- `npm run test:launch`: all six launch-settings/catalog scenarios.
- Debug Tauri build and `npm run test:app`: fake-agent terminals and the
  persistence/session/project-identity regression suite.
- Production model downloads: both exact sizes and SHA256 digests verified,
  with no partial files offered as models.
- Both Base and Small recognized the recorded WAV and inserted new text into
  the pinned fake terminal without Enter. Closing into the tray cancelled capture;
  normal process exit omitted that desk's saved screen.
- Release Tauri/NSIS build and release-executable `npm run test:app` and
  `npm run test:mcp`: fake terminal flows, saved launch overrides, session
  persistence, native stdio MCP delegation, result handoff and capability lifecycle.
- Debug and release DLL inspection: neither executable imports an unbundled
  MSVCP/MSVCR/VCRUNTIME DLL.

Cross-platform CI, all four release builds and fetch-only verification of the
signed Windows updater asset are publication gates. Their completed run results
accompany the release. Linux and ARM macOS CI compile native code and run core/UI
checks; Intel macOS is cross-compiled by the release workflow. These checks do not
exercise physical microphones or establish native desktop behavior on those OSes.

The voice extension's finish review returned `ship`. The documenter comparison
confirmed reuse of the incumbent palette, type, top bar, panel and spacing;
DESIGN.md and its sidecar were preserved. Earlier failed runs exposed an encoding
error, overlapping build/UI tests, MCP argument-boundary and macOS socket-mode
bugs, and fixture assumptions about window destruction, brief working states and
Windows short-path aliases. These were corrected and affected checks rerun.
No failed or cancelled run is counted as passing.

The Windows baseline was built from research commit
`066ee92c87c196f4abb3d135b2066dedd561ee44`: the release executable is 5,964,800
bytes and its NSIS installer is 2,459,330 bytes. The merged release also includes
main's session/reliability, launch-settings and MCP changes, so this comparison is
total release growth, not an isolated attribution to voice. Installer compression
can vary between builds and toolchains; the table records the final local build.

| Artifact / stored file | Baseline bytes | Local v0.5.0 bytes | Growth bytes |
| --- | ---: | ---: | ---: |
| Windows release executable | 5,964,800 | 7,717,376 | 1,752,576 |
| Local NSIS installer | 2,459,330 | 3,008,717 | 549,387 |
| Bundled dependency notices | — | 16,188 | — |
| Downloaded Base Q5_1 | — | 59,707,625 | — |
| Downloaded Small Q5_1 | — | 190,085,487 | — |

Executable + notices + one verified model total 67,441,189 bytes with Base or
197,819,051 bytes with Small. These are measured-file subtotals, not a measurement
of an installed directory: uninstaller, WebView2/cache, office files and filesystem
allocation are excluded. The owner's installed app was not changed. The earlier
approximately 200 MB / 70 MB descriptions remain estimates of installation use.

The official whisper.cpp v1.8.3 `samples/jfk.wav` fixture is 352,078 bytes, SHA256
`59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.
The debug-only fixture hook requires a named instance and an explicit isolated
data directory. Invalid fixtures fail without falling back to a real microphone.
Release builds do not read the fixture variable.

In the final debug fixture run, Stop-to-result was 31,790 ms for Base and
90,442 ms for Small on this Windows machine. Each is one English clip, with a
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

The fresh Windows CMake cache recorded `GGML_NATIVE`, SSE4.2, AVX, AVX2,
AVX-VNNI, AVX512 variants, FMA, F16C, OpenMP, CUDA and Vulkan as off.
`CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` and
`CMAKE_POLICY_DEFAULT_CMP0091=NEW`, together with Rust's static CRT target flag,
avoid the extra compiler-runtime DLL requirement. Both debug and release native
caches were refreshed explicitly; Cargo's package clean defaults to debug and
needs `--release` / `--target` for corresponding release artifacts.

Physical microphone permission denial, device routing/loss, old-CPU behavior and
native microphone/tray behavior on each platform remain manual checks. A real
desktop microphone smoke test needs the owner's separate explicit permission.
Models are outside the installer. Audio and recognized text stay local; model
requests go to Hugging Face. The app omits voice-used desks from its saved screen
files to avoid persisting terminal echo of a transcript.
