# Voice input repair — 10 October 2026

The owner reported that voice input did not work and its UX was poor, identifying their version as 0.5.3. The installed executable at `E:/Softwares/Agent Moshpit/agent-moshpit.exe` reports 0.5.2, and GitHub's latest published release was 0.5.2 during this work. The repair is isolated on `fix/voice-input`, based on `origin/main` at `d4e9d20`; the old `cli-office` checkout is preserved.

## Findings and changes

The original microphone was always the system default, with no device selection or input feedback. Windows currently defaults to the Razer Barracuda X headset and also lists Realtek, Steam Streaming and NVIDIA RTX Voice inputs. Enumeration was verified without opening a recording stream. The owner's saved voice settings enable Small in English; only Small is downloaded in their app data.

The original fixed 0.008 RMS cutoff rejected sustained quiet audio before inference. The repair accepts sustained speech above 0.002 RMS and normalizes quiet, accepted recordings with a maximum 20× gain. Silence, sub-threshold audio, short recordings and isolated clicks remain rejected. Streams that stop delivering samples fail with recovery instructions after three seconds, instead of appearing to listen until the minute limit. Device loss and sample-format errors identify the microphone and the next action.

The Voice button remains available for setup while voice is off. Setup shows enablement/download readiness, offers an explicit download of the selected model, and lists microphones with a refresh action. Base is the default for new settings; existing Small choices remain intact. During recording, the original terminal, microphone name, audio meter and time are visible. Transcription shows elapsed time and explains longer CPU waits. Errors have setup and retry actions. Stop restores keyboard focus after insertion unless the user moved elsewhere while waiting. Concurrent recording actions are guarded.

Audio stays local, the original terminal run remains pinned, no Enter is sent, and cancellation/window close still discard pending results. Device enumeration, setup and download never open the microphone.

## Verification

- 104 Rust tests passed; the live CLI and native-device enumeration checks are opt-in. Enumeration was separately run and passed.
- 53 frontend unit tests passed; Svelte reported no errors or warnings. Clippy passed with warnings denied.
- 227 browser checks passed, including model downloads, selected microphone feedback, cancellation, original-session insertion, shortcut behavior, narrow layout and failure recovery.
- The focused voice UX suite passed 19 checks at 1280px and 420px: setup, selected microphone persistence, silent-input feedback, retry, cancellation, viewport fit, focus restoration, preservation of a later focus choice and one insertion without Enter.
- The real debug desktop app verified both pinned model hashes and transcribed whisper.cpp's JFK WAV into an isolated fake terminal. Base took 15,504ms and Small 55,249ms from Stop to completion in this run. These are one clip on one machine, not an accuracy or latency benchmark. Closing and reopening the window cancelled capture; transcript echo was not saved.
- The Impeccable detector found one radius advisory; the meter now uses the existing pill token. Desktop and narrow screenshots were visually inspected.
- The final optimized Windows executable passed `tools/e2e/voice-setup-app.mjs`: four native inputs listed, UI and native IPC agreed, selected microphone saved, Base was the default, and setup never started a recording. The NSIS installer was built successfully (3,019,146 bytes); the executable is 7,748,608 bytes. Installer replacement was not performed.

Screenshots are from the simulated browser office, without microphone capture: [desktop setup](voice-repair-2026-10-10/setup-desktop.png), [narrow setup](voice-repair-2026-10-10/setup-narrow.png), [desktop recording](voice-repair-2026-10-10/recording-desktop.png), [narrow recording](voice-repair-2026-10-10/recording-narrow.png). They reuse the app's bundled fonts, tokens and SVG icons; no generated raster assets were introduced.

Run `npm run test:ui` for the main browser suite and `npm run build; node tools/e2e/voice-ux.mjs` for the focused recovery checks. The recorded-audio desktop check uses `tools/e2e/voice-app.mjs`, existing isolated model/WAV fixtures, a named instance and a temporary data directory.

The initial local deliverables are `.impeccable/review/voice-fix-build/Agent-Moshpit-voice-fix-test-setup.exe`, `agent-moshpit.exe` and their SHA-256 manifest in `checksums.json`. Native setup smoke checks can target the optimized executable with `MOSHPIT_APP` and `node tools/e2e/voice-setup-app.mjs`. Those initial test artifacts retain version 0.5.2. Following the owner's instruction to ship, the release is prepared as v0.5.3 with the focused browser suite on all three CI platforms and native setup coverage on Windows. Publication requires successful release CI, all platform packages and real-app update signature/tamper checks.

## Limits

The original failure has not been reproduced with the owner's physical microphone. Permission was requested for a five-second input-level check; no microphone recording was performed without a reply. Neither Windows microphone privacy access, headset mute/routing nor live speech accuracy is claimed verified. The installed app and its running agents were left in place. macOS and Linux runtime behavior was not checked on this Windows PC; their automated checks run on hosted CI.
