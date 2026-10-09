# Add local dictation across every agent

**Recommend optional local dictation using CPU `whisper-rs` plus native `cpal` microphone capture.** Offer multilingual Whisper Small Q5_1 as the balanced first model and Base Q5_1 as a lighter alternative, downloaded only after the user enables voice. This meets Agent Moshpit's free, no-account, no-billing default and works independently of each CLI's voice support. Audio stays on the computer; downloading weights still contacts Hugging Face. Recognition quality for the owner's accent, technical vocabulary, and any Hindi/Hinglish use remains unmeasured; language preference is unknown. **Choose the approach before implementation.**

Research recovered and primary sources checked **2026-10-09**. Scope: an MIT Tauri 2/Svelte 5 app hosting fourteen supported CLIs, including Claude Code and Codex, through xterm PTYs; Windows is the owner's daily platform, with macOS/Linux available through CI. Only research is complete: no engine choice has been accepted, and no app changes, model downloads, builds, tests, or microphone checks were conducted for this feature.

## Native voice covers Claude, while local apps offer alternatives

Claude Code already supports `/voice`: hold Space for dictation, or select `/voice tap` to avoid key-repeat dependence. Capture uses native modules, but **recognition streams audio to Anthropic**. It requires claude.ai authentication; direct API keys and Bedrock/other listed hosted configurations do not qualify. Transcription consumes no Claude messages, tokens, or `/usage` allowance. Eligible free/paid plans beyond the documented account requirement are unspecified. Hold mode normally waits for Enter; **tap mode can automatically submit transcripts of at least three words**. Compatibility with Moshpit's key delivery and microphone permission attribution is plausible, but untested. ([Claude documentation](https://code.claude.com/docs/en/voice-dictation))

Codex's unfinished TUI transcription prototype was removed in March 2026. An OpenAI maintainer distinguished supported desktop dictation from unsupported TUI transcription, and current CLI documentation supplies no verified supported dictation workflow. Do not revive the old experimental flag as a current solution. Claude desktop Code-tab microphone behavior is reported by users, but its official documentation did not establish a reusable dictation interface for another terminal host. Neither desktop app's microphone proves voice support in its CLI. ([Codex removal](https://github.com/openai/codex/pull/16114), [maintainer explanation](https://github.com/openai/codex/issues/16404), [current CLI guide](https://learn.chatgpt.com/docs/codex/cli), [Claude desktop documentation](https://code.claude.com/docs/en/desktop))

The named dictation products provide credible alternatives, without establishing a market leader or measured accuracy ranking. Prices below are advertised prices checked on the research date, not recommendations based on hands-on comparisons.

| Product | Desktop fit and advertised price | Processing and useful distinction |
| --- | --- | --- |
| Wispr Flow | Windows/Mac; free 2,000 desktop words/week; Pro $15/month or $12/month annually | Cloud recognition. Training, storage, history, and context controls are separate from offline processing. ([Pricing](https://wisprflow.ai/pricing), [privacy](https://wisprflow.ai/privacy)) |
| Superwhisper | Windows/Mac; unlimited free local Whisper; Pro $8.49/month, $84.99/year, or $249.99 once | Local mode fits private dictation; cloud/AI modes require separate consideration. Windows lacks some Mac features. ([Plans](https://superwhisper.com/docs/billing/plans), [Windows](https://superwhisper.com/docs/get-started/windows)) |
| Aqua Voice | Windows/Mac; free 1,000 words, not verified as recurring; Pro $10/month or $8/month annually | Cloud Avalon recognition. Advertised no-storage mode still sends audio off device. ([Pricing](https://aquavoice.com/pricing), [product](https://aquavoice.com/)) |
| MacWhisper | Mac; free tier; Pro **€64 once** | Advertises local transcription and system-wide dictation; optional cloud workflows have separate terms. ([Product/pricing](https://www.macwhisper.com/)) |
| VoiceInk | Apple Silicon Mac; $25/$39/$49 once for 1/2/3 Macs | Local transcription; optional cloud enhancement sends transcript text. Vendor site requires macOS 14.4+, while recovered repository requirements differ. ([Product/privacy](https://tryvoiceink.com/)) |
| Handy | Windows/Mac/Linux; free, MIT, Tauri | Local models, hold/toggle activation, focused-app delivery. Closest architecture reference, with Linux injection limitations. ([Repository](https://github.com/cjpais/Handy)) |
| OpenWhispr | Windows/Mac/Linux; free unlimited local/BYOK; hosted free 2,000 words/week; Pro $80/year | Local and cloud routes are selectable. ([Product](https://openwhispr.com/), [pricing](https://openwhispr.com/pricing)) |
| Whispering | Free/open source; browser plus Epicenter native host | Provider selection and optional polishing. Epicenter now owns the Tauri shell; release-platform coverage was not rechecked. ([Current README](https://raw.githubusercontent.com/EpicenterHQ/epicenter/main/apps/whispering/README.md)) |

Windows Win+H uses online Azure recognition. Copilot+ on-device cleanup does not establish offline recognition for ordinary voice typing. Apple instructs users to inspect Keyboard settings to determine local processing for their device/language. OS dictation and external apps still need insertion trials: xterm's hidden input is not an ordinary text field. Web Speech is not a verified cross-platform offline Tauri backend, and webview microphone support also varies; WebKitGTK microphone capture remains an unresolved compatibility concern. ([Microsoft](https://support.microsoft.com/en-us/accessibility/windows/use-voice-typing-to-talk-instead-of-type-on-your-pc), [Apple](https://support.apple.com/guide/mac-help/use-dictation-mh40584/mac), [Tauri maintainer discussion](https://github.com/tauri-apps/tauri/issues/13143))

## Whisper balances download size, multilingual reach, and build scope

The first choice should minimize integration scope while preserving a possible Hindi path. **Supported Hindi is not proven usable Hindi or Hinglish quality.** Engine code and model weights have different licences; the app's MIT licence does not replace dependency/model notices.

| Candidate | Code/wrapper versus weights | Decision for the first slice |
| --- | --- | --- |
| whisper.cpp / `whisper-rs` | Engine MIT; wrapper **Unlicense**; upstream Whisper code/weights MIT | Recommended CPU route; use multilingual weights, not English-only `.en`. ([Engine licence](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE), [wrapper metadata](https://crates.io/api/v1/crates/whisper-rs), [Whisper](https://github.com/openai/whisper)) |
| Parakeet / sherpa-onnx | sherpa-onnx Apache-2.0; ONNX Runtime MIT; TDT v2/v3 weights CC-BY-4.0 | Worth later English evaluation; v2 English, v3 25 European languages **without Hindi**. Int8 ONNX files total roughly 631–640 MiB uncompressed. ([Crate](https://crates.io/crates/sherpa-onnx), [runtime](https://github.com/microsoft/onnxruntime), [v3 card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3), [converted files](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/tree/main)) |
| Moonshine | Code, English weights, and streaming STT weights MIT; legacy non-streaming non-English weights have non-commercial Community terms | English Tiny/Base are 26M/58M parameters; streaming Tiny/Small/Medium 34M/123M/245M. No Hindi in current language table. ([Models](https://moonshine-voice.readthedocs.io/en/latest/models/available-models/), [licence](https://github.com/moonshine-ai/moonshine/blob/main/LICENSE)) |
| Vosk | Engine Apache-2.0; Rust wrapper MIT; listed small English/Hindi weights Apache-2.0 | Streaming alternative: advertised 40 MB English/42 MB Hindi models. Requires a native `libvosk`; laptop speed and coding-speech quality are unmeasured here. ([Models](https://alphacephei.com/vosk/models), [wrapper](https://github.com/Bear-03/vosk-rs), [engine](https://github.com/alphacep/vosk-api)) |

Pin `whisper-rs` **0.16.0**, whose published sys crate **0.15.0 vendors whisper.cpp 1.8.3**. Its archived GitHub mirror is not evidence that the current Codeberg wrapper is abandoned. Setting `WHISPER_DONT_GENERATE_BINDINGS` uses packaged bindings, so **CMake and a C/C++ toolchain remain necessary; libclang is avoidable**. Start CPU-only without GPU/OpenMP extras. Handy currently uses `transcribe-cpp` for Whisper and `cpal` for capture; that is a useful precedent, not a reason to copy its full multi-engine dependency stack. ([Wrapper README](https://codeberg.org/tazz4843/whisper-rs/src/branch/master/README.md), [published build script](https://docs.rs/crate/whisper-rs-sys/0.15.0/source/build.rs), [vendored CMake](https://docs.rs/crate/whisper-rs-sys/0.15.0/source/whisper.cpp/CMakeLists.txt), [Handy manifest](https://github.com/cjpais/Handy/blob/main/src-tauri/Cargo.toml))

| Multilingual Whisper | Standard download | Q5_1 download | Upstream rough full-model memory |
| --- | ---: | ---: | ---: |
| Tiny | 77.69 MB / 74.09 MiB | 32.15 MB / 30.66 MiB | ~273 MB |
| Base | 147.95 MB / 141.10 MiB | **59.71 MB / 56.94 MiB** | ~388 MB |
| Small | 487.60 MB / 465.01 MiB | **190.09 MB / 181.28 MiB** | ~852 MB |

Download sizes are upstream metadata; memory figures are rough full-model estimates, **not measured Q5_1 memory or total app RSS**. Quantized latency, accuracy, installer growth, and this owner's laptop performance remain unknown. ([Pinned sizes](https://huggingface.co/api/models/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1?recursive=false&expand=false), [memory table](https://github.com/ggml-org/whisper.cpp#memory-usage))

A different runtime's published Ryzen 7 PRO 4750U CPU test transcribed an 11-second JFK clip with multilingual Q8_0 Tiny/Base/Small in **0.306/0.646/2.169 seconds**, plus **0.114/0.162/0.375 seconds** loading. These are measured by its author, not Moshpit benchmarks or Q5_1 promises. Its legacy Hindi results lack sufficient provenance and show poor recognition; they support testing rather than an accuracy claim. Evaluate Indian-accent English, Hindi, mixed speech, and code terms before settling the default. ([Tiny catalogue](https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/whisper-tiny.json), [Base](https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/whisper-base.json), [Small](https://github.com/handy-computer/transcribe.cpp/blob/main/catalog/whisper-small.json))

Proposed Small download: [ggml-small-q5_1.bin](https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small-q5_1.bin), revision `5359861c739e955e79d9a303bcbc70fb988958b1`, **190085487 bytes**, SHA-256 `ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb`. Lighter [Base Q5_1](https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base-q5_1.bin): **59707625 bytes**, SHA-256 `422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898`. Hashes are upstream LFS metadata only; no download was locally hashed. ([Metadata](https://huggingface.co/api/models/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1?recursive=false&expand=false))

## Cloud costs little, but adds accounts and off-device audio

At **60 minutes/day × 30 days = 1,800 minutes = 30 hours**, the following are calculated paid equivalents before free quotas/credits, tax, retries, or text editing. OpenAI minute costs include advertised estimates; none is a measured monthly bill or latency comparison.

| Provider/model | Advertised rate | Calculated monthly cost | Setup/free boundary |
| --- | ---: | ---: | --- |
| Groq Whisper large-v3-turbo | $0.04/hour | **$1.20** | Free tier exists; account/key required. Paid Developer upgrade needs payment method. ([Speech](https://console.groq.com/docs/speech-to-text), [billing](https://console.groq.com/docs/billing-faqs)) |
| Groq Whisper large-v3 | $0.111/hour | $3.33 | Same distinction. ([Speech](https://console.groq.com/docs/speech-to-text)) |
| OpenAI gpt-4o-mini-transcribe | ~$0.003/minute | $5.40 | API account/key/billing; prepaid minimum $5. ([Pricing](https://developers.openai.com/api/docs/pricing), [setup](https://help.openai.com/en/articles/8264644-setting-up-and-managing-prepaid-api-billing)) |
| OpenAI Whisper-1 / gpt-4o-transcribe | ~$0.006/minute | $10.80 | API usage has separate billing. ([Pricing](https://developers.openai.com/api/docs/pricing)) |
| Deepgram Nova-3 monolingual files | $0.0043/minute | $7.74 | Account/key; advertised $200 no-card credit is finite. ([Pricing](https://deepgram.com/pricing)) |
| Deepgram Nova-3 monolingual streaming | $0.0048 promotional / $0.0077 regular per minute | $8.64 / $13.86 | Promotion can change. ([Pricing](https://deepgram.com/pricing)) |
| AssemblyAI Universal-2 files | $0.15/hour | $4.50 | Account/key; no-card introductory credits. ([Pricing](https://www.assemblyai.com/pricing)) |

**Groq's $1.20 is a paid equivalent, not a guaranteed free-tier bill.** Account quotas need checking; the extracted public limit table did not establish free allowances reliably. Its ten-second request minimum doubles billed duration for five-second clips. Groq advertises no inference-content retention by default with exceptions and customer-controlled ZDR, while still collecting usage metadata. AssemblyAI free users cannot opt out of model improvement or configure TTL. Endpoint-specific terms for other providers remain a gap. Cheap cloud recognition is an optional escape route for weak hardware, not the default for no-account/local use. ([Groq minimums](https://console.groq.com/docs/speech-to-text), [limits](https://console.groq.com/docs/rate-limits), [data controls](https://console.groq.com/docs/your-data), [AssemblyAI controls](https://www.assemblyai.com/docs/data-controls))

## Native capture gives Moshpit control over the destination

Propose `cpal` capture at the device's actual format, conversion to mono 16 kHz float PCM, then inference after Stop in a blocking worker. CPAL uses WASAPI/CoreAudio/ALSA on these desktop platforms; select its version deliberately rather than importing newer platform minimums accidentally. Webview PCM through `getUserMedia`/Web Audio remains feasible but adds permission differences and audio IPC. Avoid MediaRecorder codec blobs and decoding work. ([CPAL](https://github.com/RustAudio/cpal), [Tauri binary IPC](https://v2.tauri.app/develop/calling-rust/))

The proposed first slice stays **off until enabled**, with a microphone button and configurable in-app toggle shortcut. `Ctrl+Shift+Space` is a candidate subject to an app/CLI conflict audit. Show **blue listening state**, elapsed time, Stop/Cancel, and a distinct transcribing state; begin listening only when capture is ready. Bound capture to 60 seconds and cap buffering. Reject short/silent recordings before inference; add speech detection if energy filtering proves insufficient. Silence rejection reduces hallucination risk without proving none occur.

Lock the target pane, session ID, and generation at recording start. Insert only if that same session remains live, even if focus changes. Flatten newlines and remove terminal control sequences/control characters before using the existing paste path. **Never inject Enter or auto-submit.** Cancel and discard on explicit cancellation, pane close, restart, or webview closure into tray; an unseen microphone must not continue recording while the backend survives. Keep callbacks bounded and free of inference, disk, and network work. Device denial/disconnection and inference errors must leave recording stopped with a recoverable error.

Voice settings should expose enable/disable, model size/download status, language selection, and deleting downloads. Language preference remains unknown; offer explicit English/Hindi selection without promising mixed-language quality. Disclose Hugging Face before download, keep models in Moshpit's own data directory, verify pinned size/hash, and atomically promote a partial file. Avoid cloud rewriting, clipboard use, and transcript/audio persistence in this slice. This delivery plan is a proposal, not verified behavior.

Linux needs ALSA development headers and compatible runtime audio libraries. macOS needs `NSMicrophoneUsageDescription` and the applicable audio-input entitlement; native capture still requires OS consent. Configure these through Tauri's bundle settings. CPU release flags must match the supported baseline: disabling `GGML_NATIVE` alone is insufficient evidence that AVX/AVX2 assumptions are gone. Inspect the pinned vendored ggml options and test that baseline. Static inference code will increase installers even when weights are unbundled; the amount is unmeasured. ([CPAL prerequisites](https://github.com/RustAudio/cpal), [Apple usage description](https://developer.apple.com/documentation/bundleresources/information-property-list/nsmicrophoneusagedescription), [entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.audio-input), [Tauri configuration](https://tauri.app/reference/config/#infoplist), [ggml option precedent](https://github.com/ggml-org/whisper.cpp/blob/v1.9.5/ggml/CMakeLists.txt))

## Owner selection precedes a bounded implementation and validation

After selection, make the proposal reviewable through these areas, adjusting exact hooks after inspecting lifecycle and terminal insertion code.

| Area | Proposed change |
| --- | --- |
| `App.svelte`, `TerminalView.svelte`, panes, `AppMenu.svelte`, new Voice panel | Voice states, selected-pane targeting, mic controls, shortcut, settings, lifecycle cancellation |
| `src/lib/bridge.ts`, `demo.ts` | Typed voice commands/events and deterministic fake recording/transcription |
| Rust voice module, settings, `lib.rs` lifecycle | Capture/resampling, model verification, worker, bounded state, session validation, shutdown |
| CI, Tauri bundle config, documentation/notices | CMake/packaged bindings, Linux audio prerequisites, macOS permissions, CPU baseline and licences |

Validate state transitions, sanitization, bounds, and stale-result rejection with pure tests; exercise the UI through the fake demo. Use recorded WAV clips and a fake terminal for transcription/delivery checks without activating a microphone or real agents. Compile/check on Windows/macOS/Linux, then run **explicitly opt-in native microphone smoke checks** on available hardware. CI compilation does not establish microphone consent or audio routing. Measure warm/cold latency, RSS, installer growth, and errors on 5/15/60-second clips, silence, technical vocabulary, and the owner's preferred language before declaring the default successful. No deployment, push, or release belongs to this proposal.

## Conclusion

The useful product outcome is reliable insertion into the intended agent prompt with local processing and an explicit sending step. Recognition benchmarks alone cannot validate that behavior; pane lifecycle and the owner's actual speech must become acceptance gates.

| Owner choice | Consequence |
| --- | --- |
| **Integrated local voice — recommended** | Build the bounded `whisper-rs`/`cpal` slice; evaluate Small Q5_1 versus lighter Base Q5_1 in the preferred language. |
| Use existing tools | First test Claude `/voice` or a local dictation app inside Moshpit; adopt the working option without an integrated engine. |
| Optional cloud | Consider Groq or another API only if sending audio off device and account/key setup are acceptable. |
