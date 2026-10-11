# Voice engines, October 2026

**Decision (2026-10-12, shipped in 0.7.0):** voice is English only. It turns speech into words either on the computer, with NVIDIA's Nemotron Speech Streaming EN 0.6B run by transcribe.cpp in a helper process, or with a cloud provider the user picks and pays with their own key: Deepgram, OpenAI, AssemblyAI or ElevenLabs. Whisper and the language setting are gone.

## Why

The owner found 0.6's recognition "damn bad" and its streaming "not that seamless", and asked for bring-your-own-key and NVIDIA Parakeet or similar models. The cause was the engine, not its tuning. Whisper reads 30-second windows, so 0.6 faked streaming by reading the current phrase again about twice a second and typing it at each pause: the shown words changed while you spoke, and text arrived in bursts. Base and Small, the only Whisper sizes fast enough on a laptop CPU, were weak on accents and code words.

The bar is the owner's: words appear while you speak, and finishing takes well under a second, the way dictation feels in Claude Desktop.

## The local engine

The candidates ran on the owner's computer (Ryzen 5 5600H, 6 cores) through the transcribe.cpp 0.3.1 streaming API with the same build flags the app ships (AVX2, no AVX-512, static runtime). Each clip was fed in 20 ms pieces as fast as the engine took them, and the times are relative to the clip's start. The clips are one recorded clip (whisper.cpp's `jfk.wav`) and six code-dictation commands spoken by Windows text-to-speech voices (Ravi and Heera, Indian English; Mark and Zira, US English), 7 to 11 seconds each, plus all six joined (35 s) and a fan-noise clip.

| Model and setting | Speed (× real time) | First text | Typed text trails the audio by, at most | Stop to last words |
| --- | --- | --- | --- | --- |
| Parakeet Unified 0.6B Q8, streaming (70/7/7) | 0.9–1.3 | 1.9 s | 0.7–6.0 s | 1.3–6.7 s |
| Parakeet Unified 0.6B Q8, whole clip at once | 9.5–9.6 | after stopping | — | 1.1 s for 11 s of audio, 3.7 s for 35 s |
| **Nemotron Streaming Q8, right context 6, 4–6 threads** | **5.3–6.2** | **0.6–1.2 s** | **0.11–0.18 s** | **0.09–0.12 s** |
| Nemotron Streaming Q8, right context 1 | 2.9–3.0 | 0.65–0.95 s | 0.06–0.11 s | 0.05–0.13 s |
| Nemotron Streaming Q8, right context 13 | 6.0–6.5 | 1.3 s | 0.19–0.21 s | 0.11–0.19 s |
| Nemotron Streaming Q4_K_M, right context 6 | 5.8–6.5 | 0.6–1.8 s | 0.11–0.13 s | 0.09–0.12 s |

- **Parakeet Unified** is the more accurate family, but streaming it costs about as much computing as the audio lasts on this processor, so on a long sentence the text falls seconds behind. Read as a whole after stopping, it is fast, but then nothing appears while you speak. Neither meets the bar.
- **Nemotron Speech Streaming** is built for this. It is a cache-aware Parakeet-architecture model whose words, once given, are never taken back. Every word was typed before stopping, and the last ones came about a tenth of a second after. Its look-ahead ("right context") sets how often words come and what they cost: at 6 (480 ms) they come in 0.56 s steps for about a sixth of the time the audio lasts; at 1 (80 ms) in 0.16 s steps, about 0.3 s sooner, for twice the processor; 13 is only a little lighter than 6 and trails further. The spike picked 6; the release build's numbers below moved the app to 1, with up to 4 threads.
- **Accuracy:** on the joined 35-second clip, Nemotron Q8 made 1 error in 81 words ("at a unit test" for "add a unit test"), counting words after dropping case and punctuation. It also wrote "the voice engine's branch" for "the voice engines branch", which that counting does not see. It spelled "getUser" as "get user" or "Get User", as the reference does. On the fan-noise clip it wrote nothing. These are text-to-speech voices and seven clips, not a benchmark of accents or real microphones.
- **Memory:** about 1.08 GB at the peak with Q8 and 0.84 GB with Q4_K_M. The model loads in about 0.8 s.
- **Files:** Q8_0 is 729,650,176 bytes and Q4_K_M 475,436,032 bytes, from `handy-computer/nemotron-speech-streaming-en-0.6b-gguf`, pinned to revision `9789e0eb`, with their SHA-256 values in `src-tauri/src/voice/models.rs`. The model is licensed by NVIDIA Corporation under the NVIDIA Open Model License, which allows commercial use; the app does not distribute the model, and names the licence in its notices.

**Gate A, transcribe.cpp or sherpa-onnx:** transcribe.cpp built on Windows with the static runtime, and passed the check that the program needs no Microsoft C++ runtime DLL. It runs the model above, and it is the engine the Handy dictation app (also Tauri) ships, so sherpa-onnx was not needed.

### In the app

The engine runs in `agent-moshpit --voice-worker`, a child process of the app itself, so a crash in native code ends one recording, never the office and its terminals. The model loads while the microphone opens, and what is said meanwhile is kept. It stays loaded for five minutes after the last recording. With the debug build, the desktop test (`tools/e2e/voice-app.mjs`) fed `jfk.wav` at speaking pace:

- "fellow Americans" (said by 2.3 s) was typed 2.9–3.8 s after listening began, with the model loading cold. It was typed while the recording was still listening.
- After stopping at 11.5 s, the rest was typed and the recording was idle 0.23–0.24 s later.
- Killing the worker mid-sentence ended that recording with a plain message, and the next one started a new worker.

### The release build

The release program's own worker (`agent-moshpit --voice-worker`), driven the way the app drives it: the model loaded once, then each clip fed in 20 ms pieces at speaking pace, then finished. Times are from the clip's start, on the same 5600H, Q8 model, 4 threads. "Processor" is the worker's processor time over the clip's length: 1.0 is one of the 5600H's twelve logical processors, kept busy.

| Right context | First words typed | Words arrive | Stop to last words | Processor | Errors (8 clips, 217 words) | Noise (fan, hum, white, silence) |
| --- | --- | --- | --- | --- | --- | --- |
| 6 | 0.62–1.37 s | every 0.56 s | 0.09–0.14 s | 0.65–0.78 | 2 | nothing typed |
| **1 (shipped)** | **0.62–0.94 s** | **every 0.16 s** | **0.05–0.10 s** | **1.17–1.37** | **2** | **nothing typed** |
| 0 | 0.62–0.94 s | every 0.08 s | 0.04–0.06 s | 1.98–2.24 | 0 | nothing typed |

- Words were typed within 13 ms of the audio that settled them reaching the worker; the wait is the model's look-ahead, not the computer.
- Peak memory was 1.13–1.15 GB with Q8. Loading took 0.7–1.9 s; the slower load was the first read of the file from disk after building.
- Both errors at 6 and at 1 were "at a unit test" for "add a unit test" (in its own clip and in the long one); 0 heard it right. At 1 and 0 the last sentence sometimes lacks its full stop.
- The shipped setting uses about 11% of this processor while dictating, against a budget of 50%, and finishes well inside the 0.8 s the plan allowed. First words come 0.6 to 0.9 s into a sentence, which is at or a little over the 0.7 s the plan hoped for. Right context 0 brings them no sooner and costs 60% more processor while agents are building beside it; it is a one-line change if a real microphone shows its accuracy matters.

## Cloud, with the user's own key

All four providers the owner chose stream over a WebSocket, which the app opens from Rust (the window's content policy blocks sockets, and the key must not reach the window). Each provider is a small state machine with no I/O of its own (`src-tauri/src/voice/cloud/`), driven by one thread per recording that sends 100 ms of audio at a time.

| Provider and model | Words shown while speaking | Typed | Finishing |
| --- | --- | --- | --- |
| Deepgram Nova-3 (`/v1/listen`) | interim results | each final result | `Finalize`, then `CloseStream` |
| OpenAI `gpt-live-transcribe` (`/v1/realtime?intent=transcription`) | transcript deltas | each completed item, in commit order; the app commits at pauses | commit the rest, then clear |
| AssemblyAI Universal-Streaming English (`/v3/ws`) | the turn's words | each formatted turn | `ForceEndpoint`, then always `Terminate` (it bills for the time the socket is open) |
| ElevenLabs Scribe v2 Realtime | partial transcripts | each committed transcript; the app commits at pauses | a last chunk with `commit` |

- The wire formats were checked against each provider's documentation and official SDK source, and each was probed with a wrong key to record how it refuses (October 2026). Deepgram refuses before the socket opens; the others open it, send an error, and close.
- Each is asked for English. Deepgram is asked not to use the audio to improve its models (`mip_opt_out=true`). OpenAI's audio goes as 24 kHz, the only rate it takes. ElevenLabs keeps request logs unless the plan has zero retention, and charges extra for word hints; the panel says both.
- **Word hints:** the user's own words, program names, and (unless turned off) the desk's name, folder, repository and branch, cut to each provider's limits: Deepgram 100 terms and about 500 tokens, AssemblyAI 100 terms of 50 characters, ElevenLabs 50 terms of 20 characters.
- **Keys:** checked by a free call (Deepgram's projects, AssemblyAI's transcript list, the OpenAI and ElevenLabs model lists), then kept in the system keychain (`keyring` 3.6: Credential Manager, the macOS keychain, the Secret Service). Only the last four characters and when the key was added are written to `voice-keys.json`, so drawing the panel never opens the keychain. Without a keychain the key can be kept until the app closes, and never in a file. The key is read when a recording starts and goes only into the provider's header.
- Against a pretend Deepgram on this computer (`tools/e2e/voice-mock.mjs`), with the debug build, words were typed while listening and the last ones 0.11 s after stopping. A rejected key, a dropped connection and a cancelled recording each behaved as the panel describes.

No provider was used with a real key before release: the owner chose to ship 0.7.0 without that check (2026-10-12). So each adapter is proven against its documentation, its refusal of a wrong key and the pretend server, not yet against a real recording.

## What is not established

- How well either engine hears real people, on real microphones, in a room. The clips are text-to-speech voices.
- Each cloud provider's real latency and behaviour with a real key (not checked before 0.7.0, and they may change).
- macOS and Linux beyond CI building them and starting the worker.

## Also considered

- **A larger Whisper (large-v3-turbo):** it still reads 30-second windows, so streaming would still be faked, and it is several times the size of the Base and Small that 0.6 could afford on a laptop CPU.
- **Kyutai STT, Moonshine and sherpa-onnx streaming Zipformer:** looked at, not measured. No CPU figures for Kyutai's streaming models were found. Moonshine's streaming English models and Zipformer run on a CPU, but no accuracy figures comparable with Nemotron's were found either. Nemotron was measured first and met the bar.
- **A cloud provider by default, or as a fallback:** not chosen. Local is the default, and the cloud is used only when the user picks a provider and adds their own key.
