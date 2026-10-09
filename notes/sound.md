# Sound (card basic-sound)

`lib/sound.warp` (sample: samples/music.warp), loaded without `use` when a program calls play, melody, tone or beep (src/modules.rs SOUND_WORDS):

```warp
beep
play 440Hz for 0.5s
play C4                         // note_seconds (0.5 s)
play [C4 E4 G4] for 1s          // a chord
melody [C4 D4 E4]               // one after another
melody [G4 E4 C4] each 200ms    // or melody([G4 E4 C4], 200ms)
tone(220Hz, 300ms, "square")    // sine, square, triangle, sawtooth
tone 330Hz for 0.1s
play note("F#4") for 250ms      // note(name) needs `use sound` when called alone
```

- The samples are made in warp (22050 Hz, 16-bit mono, 5 ms fade at both ends) and cross to the host word
  `sound_samples(samples, count, rate)` as whole numbers ≥ 0 (amplitude + 32768), read in one call by list_to_ints.
- Natively (src/sound.rs) a WAV in `$TMPDIR/warp-sound/` played by afplay/paplay/aplay to its end; under tests, CI and
  WARP_NO_WINDOW only the file and a `sound 0.50 s: <path>` line on stderr: tests stay silent.
- Browser: the worker posts `{type: "sound"}`, the page (playground.js playSound) plays it with WebAudio, queued one
  after another; a new run silences what still plays (typing reruns the code). A browser starts audio only after a
  gesture: the run on page load stays silent, ▶ plays. A run without page hooks (tests) is silent.
- Music files (card sound-library): `play "song.mp3"` / `play_file(path)` play a wav, mp3, ogg, flac, aiff or m4a in
  the background (format by its first bytes), `stop_sound` stops them. Natively (src/sound.rs play_file) the first
  player the system has for the format: afplay (no ogg), paplay (no mp3), ffplay, mpv; headless only the check and a
  `sound file <format>: <path>` line (tests/programs/test_sound_files.rs). In the playground an `<audio>` by its URL
  (host-files.js STD_ADAPTERS.sound → worker → playground.js playSoundFile); a new run stops it. Card sound-file-failure: the
  file is decoded first by ffprobe or afinfo (a broken one fails play, also headless); a player failing within 0.3 s
  fails play, a later failure prints `sound file <path>: <player> failed (<status>): <its error output>`; without a
  decoder on the machine only the format is checked. In the browser a failure reaches the console.
- Units: `Hz` is `1/s`, `kHz` `1/ms` (src/units.rs UNIT_ALIASES); `si_amount(x)` takes a quantity or a plain number.

Limits found on the way: assignments to the module's globals (`note_seconds = 0.25`) from the program do not
reach the module.

## What professionals expect (user question 2026-10-09; roadmap, nothing of it built yet)

The words above are layer 1, the toy layer. Each layer below keeps the ones above working and lowers to them.

1. Non-blocking, clocked playback. Today `play` renders the whole sound and blocks (natively afplay runs to its end).
   Expected: a shared audio clock, sample-accurate scheduling (`at 2 beats play C4`), `play` returns a handle (stop,
   ramp its gain), voices overlap. Tempo as a unit: `bpm`, `beat`, `bar` (`play C4 for 1/4 beat`).
2. Music values, not frequencies. `Note` (pitch class, octave, MIDI number, cents), `Interval`, `Chord(C4, major7)`,
   `Scale(D, dorian)`, transposition, tuning (A4 = 442Hz, just intonation), velocity. Note names parsed, not a table.
3. Synthesis. Oscillators (sine, saw, square, triangle, noise, wavetable, band-limited, detune, unison), ADSR
   envelopes, filters (lowpass/highpass/bandpass with resonance), LFOs and parameter automation (ramps), gain in dB,
   stereo pan, polyphony with voice stealing.
4. A signal graph. `osc(220Hz, saw) |> lowpass(1.2kHz, q: 4) |> delay(3/8 beat) |> reverb(0.3) |> gain(-6dB) |> out`:
   nodes and buses as values, effects (delay, reverb, compressor, distortion, EQ), a mixer.
5. Samples and files. Load WAV/MP3/OGG/FLAC, play at a pitch or rate, slice, loop points; render offline to a file
   (`render 8 bars to "song.wav"`), which is also how tests check audio without speakers.
6. Patterns and sequencing. Sonic Pi `live_loop`, Tidal mini-notation (`"c4 e4 [g4 b4]*2"`), Euclidean rhythms,
   swing, randomness with seeds; live coding: redefine a loop while it plays.
7. MIDI and devices. MIDI in/out (CoreMIDI, Web MIDI), choosing the output device, channels, buffer size and latency.
8. Real-time DSP in warp itself. A user-written `process(block)` compiled to wasm runs in an AudioWorklet in the
   browser and in a CoreAudio render callback natively: the same code both ways, which is where wasm-first warp beats
   other languages. Needs f32 arrays without boxing (today samples cross as a list of Ints), no allocation in the
   callback, 44.1/48 kHz stereo.

Order of value: 1 + 5 (handles, clock, offline render) first, they change the architecture; then 2, 3, 4; 8 last.

## Our own API or the platform's? (card our-sound)

Recommendation: warp's own small vocabulary on top, Web Audio's model underneath, the platform APIs only as the
last stage that moves samples to the speaker.

- **Web Audio is the model to stay close to.** Its concepts (an AudioContext with one clock `currentTime`, nodes
  connected into a graph, AudioParams with scheduled ramps, AudioBuffers, AudioWorklet for custom DSP) are the
  common ground every modern system converged on, and the playground must run on it anyway. Layers 1, 3, 4 and 8
  map one to one: `osc |> lowpass |> out` is OscillatorNode → BiquadFilterNode → destination; a handle's `ramp` is
  `linearRampToValueAtTime`; `process(block)` is an AudioWorkletProcessor. Copying its names where they are good
  (gain, frequency, detune, Q) keeps warp code readable to anyone who knows the web.
- **But not its API shape.** Web Audio is verbose (`ctx.createOscillator()`, `.connect()`, `.start(t)`) and has no
  music values. warp keeps its own words: units (`440Hz`, `200ms`, `-6dB`, `1/4 beat`), notes and chords as values,
  `play … for …` phrases and `|>` pipes. Those lower to the graph; a program never sees a context object.
- **Natively, the same graph runs in our own engine, written in warp/wasm** (layer 8): it renders blocks of f32
  samples, so the browser and the Mac produce the same samples and offline rendering (tests!) is the same code path.
  Only the output differs:
  - macOS: **CoreAudio** — AudioUnit/AudioQueue with a render callback (low latency, the device's rate, typically
    48 kHz, 128–512 frame buffers); AVAudioEngine is the higher-level Swift/ObjC wrapper (graph of nodes, players
    for files, effects — Apple's own Web-Audio-like model) and AVAudioPlayer plays whole files. CoreMIDI for MIDI.
  - Linux: **ALSA** is the kernel-level PCM interface (open `default`, write interleaved frames); on desktops the
    sound server sits on top: **PipeWire** (now standard, also speaks the PulseAudio and JACK protocols), formerly
    PulseAudio; JACK for pro low-latency routing. A program should talk to PipeWire/PulseAudio or ALSA `default`,
    never pick one hardware device itself.
  - Windows: WASAPI.
  - The Rust crate **cpal** wraps all of these (CoreAudio, ALSA, PipeWire via ALSA/JACK, WASAPI, even Web Audio) with
    one callback interface: the natural native backend for warp's engine. **rodio** (on cpal) decodes WAV, MP3,
    OGG/Vorbis, FLAC and mixes sources: the shortest path to layer 5 file playback. **symphonia** decodes more formats.
- **Today's stopgap** (a WAV file handed to afplay/paplay/aplay) stays the fallback when no audio device is reachable
  and for CI, where the file itself is what tests check.

So: Web Audio semantics, warp words, one engine for rendering, cpal (CoreAudio/ALSA/PipeWire/WASAPI) or Web Audio's
destination for output. Not a wrapper over each platform's own high-level API (AVAudioEngine, GStreamer): those
differ in model and would make the browser and native sound differ.
