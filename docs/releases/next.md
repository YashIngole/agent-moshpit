# Next release (unreleased)

- Optional local voice input for any live terminal, off until enabled. Start and stop with the mic or the configurable in-app shortcut. Review the inserted text and press Enter yourself to send it.
- Download multilingual Small Q5_1 (190.1 MB) or lighter Base Q5_1 (59.7 MB) explicitly from Hugging Face, outside the installer. Downloads are checked against their pinned size and SHA-256; models can be removed in **More → Voice input**.
- Audio and transcription stay local, without an account, billing, cloud rewriting or telemetry. Recordings are bounded to 60 seconds and stay in memory. Closing a pane, restarting its program, cancelling, minimising or closing the window discards pending voice input.
- Speech quality for accents, Hindi/Hinglish and technical vocabulary is still unmeasured. Native microphone permission, routing and disconnection checks remain manual; see [voice validation](../reviews/voice-validation-2026-10-09.md).
