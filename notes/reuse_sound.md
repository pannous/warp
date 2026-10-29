# Reusing existing work for sound (card reuse-sound, user policy reuse-giants 2026-10-29)

What we build ourselves today, and which existing library covers it instead. Since the Rust harness went (11:00,
2026-10-29), the native warp plays sound only as a WAV handed to afplay/paplay/aplay. Fifteen sound and MIDI host
words of lib/sound.warp (`std_io("sound", …)` and `std_io("midi", …)`) answered from Rust (cpal mixer, midir,
effects.rs) are unanswered natively now: stop, fade, bus, send, blocks, live_cycle, spectrum, file_spectrum,
file_seconds, and every midi one. That is the gap the choices below fill.

## Verdicts

| Layer | Take | Binding | License | We stop building |
|---|---|---|---|---|
| Device output, mixing, resampling | **miniaudio** (mackron, one C file, 0.11.21) | C ABI, one small wrapper dylib beside libwarp_wasmtime (branch sound-plays: crates/warp-sound) | public domain or MIT-0 | the WAV-to-afplay path (kept as fallback), cpal's mixer, the Rust voice clock's output side |
| Fades, gain, pan, pitch, groups (buses) | miniaudio's `ma_sound` / `ma_sound_group` (fade in/out ms, volume, pan, pitch, start/stop at a PCM frame) | same wrapper, a few more C functions | same | `stop`, `fade`, `bus` host words; sample-accurate `at` scheduling (ma_sound_set_start_time_in_pcm_frames) |
| Filters and EQ | miniaudio `ma_lpf`/`ma_hpf`/`ma_bpf`/`ma_peak`/`ma_loshelf`/`ma_hishelf`, `ma_delay` as nodes in `ma_node_graph` | same | same | the bus side of effects.rs (the per-note biquads in warp stay: they run in the playground too) |
| Reverb | **verblib** (Freeverb as one C header), or miniaudio's extras reverb node built on it | compiled into the same wrapper | public domain / MIT-0 | REVERB_DELAYS echo approximation on buses |
| Decoding WAV, MP3, FLAC | miniaudio itself (dr_wav, dr_mp3, dr_flac inside) | same | same | afinfo/ffprobe probing, afconvert/ffmpeg conversion in `sample(path)` and `play "song.mp3"` |
| Decoding OGG Vorbis | stb_vorbis (miniaudio's extras hook it in) | same | public domain / MIT | ffplay/mpv for .ogg |
| AAC, ALAC (m4a), Opus, and decoding in the browser | **symphonia** compiled to a wasm component (`decode: func(bytes) -> result<pcm, string>` in WIT) | WIT component, the same in the playground and natively | MPL-2.0 (file-level copyleft: fine to ship unmodified) | the playground's WAV-only decoder; the m4a gap natively. Optional: only when a program plays those formats |
| MIDI devices | **RtMidi** (its C API rtmidi_c.h) or **PortMidi** (C) | C ABI, `from 'rtmidi'` lines in lib/sound.warp, or the same wrapper | RtMidi MIT-style, PortMidi MIT-style | midir (gone); `midi_send`, `midi_inputs`, `midi_outputs`, `midi_receive` |
| MIDI files | keep ours: the codec is small and already in warp's style; host-files.js has its JS twin | none | — | — (only `read_file`/`write_file` move from std_io into plain warp bytes) |
| FFT for spectrum | **pocketfft** (C) or KissFFT, only if the per-band DFT is too slow | C ABI | BSD-3 | nothing yet: `spectrum` is plain warp and runs in the playground; file_spectrum becomes `spectrum(sample(path), …)` |
| Synthesis, patterns, live loops | keep ours: they are the language's value (notes, units, `|>` pipes) and must run identically in the browser | — | — | — |

## Not taken
- **PortAudio, libsoundio, SDL3 audio**: device output only; miniaudio does output plus decoding, mixing and nodes in
  one file without a build system. SDL3 is worth it only if reuse-graphics picks SDL3 for windows anyway: its audio
  stream API then comes free, but still without decoding or nodes.
- **FAUST**: a DSP language compiling to C or wasm; powerful for synth and effects, but a second language and toolchain
  beside warp. Revisit for a pro-audio layer.
- **Strudel / Tidal code**: the model for our pattern mini-notation, but Strudel is AGPL-3.0; keep ours.
- **Web Audio in the playground** stays: it is the browser's miniaudio.

## Shape
One wrapper library, libwarp_sound, grows from `warp_sound_play(path)` to the handful of calls the unanswered host
words need (play PCM from memory with a start frame, stop, fade, group, node chain, MIDI out/in), each a plain C
function lib/sound.warp declares `from 'warp_sound'`. The std_io calls then become those foreign calls, and the
native warp needs no sound code of its own. The open question is core's: the driver resolves C functions at load
time, so an optional library needs the call-time failure on branch sound-plays (asked of the Interviewer).
