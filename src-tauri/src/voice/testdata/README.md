# Voice test audio

Short excerpts of `samples/jfk.wav` from whisper.cpp v1.8.3 (MIT; the speech is
President Kennedy's 1961 inaugural address, a US government work in the public
domain). Raw 16-bit little-endian mono PCM at 16 kHz, named by their start and
end in seconds. Used only by tests; nothing here is built into the app.

The source file's SHA-256 is
`59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.

- `jfk-1.40-2.00.s16` and `jfk-9.60-10.20.s16`: speech with no pause around it,
  which an earlier speech check took for background noise.
- `jfk-7.65-8.15.s16`: the half-second of the sample whose level varies least,
  the hardest real speech for the steady-noise check.
