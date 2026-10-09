# Local voice validation — 2026-10-09

This implements the owner's accepted local integration from the
[research record](../research/voice-to-text-2026-10.md). Voice is optional and off
by default. Pinned whisper-rs/whisper.cpp runs on the CPU; CPAL captures the default
native microphone. Models download only on explicit action, into app data.

## Verification record

The final merged-tip checks and release measurements are being completed for
v0.4.0. This record will be updated before publication; pending checks are not
counted as passing.

The Windows baseline was built from research commit
`066ee92c87c196f4abb3d135b2066dedd561ee44`: the release executable is 5,964,800
bytes and its NSIS installer is 2,459,330 bytes. Final artifact growth remains to
be measured. The approximately 200 MB / 70 MB descriptions discussed with the
owner were estimates, not installation measurements.

The official whisper.cpp v1.8.3 `samples/jfk.wav` fixture is 352,078 bytes, SHA256
`59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.
The debug-only fixture hook requires a named instance and an explicit isolated
data directory. Invalid fixtures fail without falling back to a real microphone.
Release builds do not read the fixture variable.

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
