# Sound

Roadmap card: sound (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

`lib/sound.warp` (sample: samples/sound/music.warp) is loaded without `use` when a program calls play, beep,
stop_sound or wait_sound (src/modules.rs SOUND_SETUP_FREE_WORDS). Every other sound word (melody, tone, pattern,
live_loop, render_sound, sound_queued, play_file, the midi words) needs `use sound`; called bare it is the error
naming the use (P171).

```warp
use sound
beep
play 440Hz for: 0.5s
play C4                         // note_seconds (0.5 s)
play [C4 E4 G4] for: 1s          // a chord
melody [C4 D4 E4]               // one after another
melody [G4 E4 C4] each: 200ms    // or melody([G4 E4 C4], 200ms)
tone(220Hz, 300ms, "square")    // sine, square, triangle, sawtooth
play note("F#4") for: 250ms
```

A module's `global` is a setting: the program's top-level `note_seconds = 0.25` or `tempo = 90` sets it
(modules.rs shadowed_apart leaves globals alone). Units: `Hz` is `1/s`, `kHz` `1/ms` (src/numbers/units.rs
UNIT_ALIASES); `si_amount(x)` takes a quantity or a plain number.

## Probes and quiet runs
- Run sound probes under WARP_NO_WINDOW=1: otherwise sound plays on the user's Mac. Under tests, CI and
  WARP_NO_WINDOW nothing plays; a `sound 0.50 s: <path>` line goes to stderr and the WAVs are still collected for
  render_sound.
- probes/sound/sound_words.warp exercises the words; probes/music_go/ the `play`/`melody` phrases;
  probes/music_outlives_file.py that the player stops with warp.
- A loop variable named `note` inside lib/sound.warp breaks: `note` is the module's own word.
- A lib parameter named `s` (a unit) shadows `0.5s` program-wide, hence names like `clip`.

## Output
- Samples are made in warp (22050 Hz, 16-bit) and cross to the host word `sound_samples(samples, count, rate,
  channels)` as whole numbers ≥ 0 (amplitude + 32768), read in one call by list_to_ints.
- Native output (src/sound/sound_engine.rs, tests/programs/test_sound_engine.rs): on macOS every sound goes into one
  cpal output stream (CoreAudio). A `Mixer` holds the sounds, each converted to the device's rate and channels,
  starting at its frame; the device callback `fill` mixes them (never waits: `try_lock`, silence when a queueing sound
  holds the lock). The frame is (instant + 60 ms lead − stream start) × rate, so back-to-back sounds stay contiguous
  to the sample and `at 2 beats` lands to the sample live. At most 32 sounds at once (MAX_VOICES, playground.js
  stealFor): one more steals the oldest, which fades out in 5 ms. No output device → a WAV in `$TMPDIR/warp-sound/`
  (one name per process, deleted once played) played by afplay/paplay/aplay. Not yet: Linux/Windows output (only the
  macOS cpal dependency is declared).
- Browser: the worker posts `{type: "sound"}`, the page (playground.js playSound) plays it with WebAudio at its
  `at`; a new run silences what still plays. Audio starts only after a gesture: the run on page load stays silent.
- Music files: `play "song.mp3"` / `play ./mozart.mp3` / `play_file(path)` play wav, mp3, ogg, flac, aiff or m4a in
  the background (format by its first bytes). Natively the first player the system has for the format: afplay (no
  ogg), paplay (no mp3), ffplay, mpv; the file is decoded first by ffprobe or afinfo (a broken one fails play, also
  headless), a player failing within 0.3 s fails play, a later failure prints `sound file <path>: <player> failed
  …`. In the playground an `<audio>` by its URL (host-files.js STD_ADAPTERS.sound). ctrl-c and Sublime's cancel stop
  warp and the player together (tests/programs/test_sound_files.rs).

## Layers
Each layer keeps the ones above working and lowers to them. Order of value: 1 + 5 first, then 2, 3, 4; 8 last.

1. **Non-blocking, clocked playback** (built). `play` returns at once, its sound queued on the audio clock behind the
   sounds before it; `sound_queued()` the seconds still to sound, `wait_sound`, `stop_sound` drops the queue, the
   process's end waits (also for music files).
   - Handles: `play`, `melody`, `tone`, `play "song.mp3"` give the sound's number in the run (from 1);
     `stop_sound(h)` drops or stops that one, `stop_sound` all (tests/programs/test_sound_handles.rs).
   - Offline render: `save sound as: "song.wav"` / `save(sound, "song.wav")` (the word sound is a lib Recording,
     `render_sound` the old name with a note) writes the sounds since the last render
     into one WAV, mixed at their logical places at the highest rate among them, and gives its seconds; headless too,
     so tests check audio without speakers (tests/programs/test_sound_render.rs). Kept in memory (the $TMPDIR folder
     is shared by all warp processes). The playground refuses it.
   - Tempo: `beat(1/4)`, `beats 2`, `bar 1` are seconds at the module's `tempo` (120) and `beats_per_bar` (4)
     (lib/sound.warp beat_seconds, bar_seconds; tests/programs/test_sound_tempo.rs). No SI unit.
   - Voices: every task (`go { }`) is a voice forking its starter's place in time (src/sound/mod.rs Voice), so a
     task's notes sound along with the starter's (tests/programs/test_sound_voices.rs). In the playground each Worker
     keeps its voice's end on a shared clock (host-tasks.js voiceSounded); a task's sounds reach the page once the
     program's Worker is back in its event loop (after `await`) and go back with its result (withTaskSounds).
   - Scheduling: `at 2 beats play C4`, `at 1 bar { … }` (sound_at / sound_at_end): its sounds start at that time
     counted from the voice's first sound, then the voice goes on where it stood
     (samples/sound/arrangement.warp).
   - Pacing: heard, a voice's next sound waits until its queued sounds end within PLAY_AHEAD (0.5 s; sound.rs paced,
     host-files.js holder.sounded), except inside an `at`. In the browser a paced endless loop with tasks hears the
     main voice only (a MessagePort per task Worker would fix it).
   - `fade_sound(handle, gain, seconds)` ramps a playing sound's gain linearly (0 = all handles), natively, in the
     browser (a GainNode per sound) and in render_sound (tests/programs/test_sound_fades.rs).
2. **Music values** (built, tests/programs/test_sound_notes.rs, test_music_values.rs, test_note_values.rs).
   - Note names are parsed, not a table: a letter A–G, `#`/`♯` or `b`/`♭`, an octave 0–9, when the program uses sound
     and doesn't define that name; `F#4` parses as `F # 4`. A name lowers to `midi_hertz(midi)`, so `tuning = 442`
     tunes every note. Bare names stay frequencies (`C4` is 261.63).
   - `midi(pitch)`, `note_name(pitch)`, `transpose(pitch or list, semitones)` (keeps a note a note, multiplies any
     other frequency), `chord(root, kind)`, `scale(root, kind)` (kinds in lib chord_intervals / scale_intervals, as an
     undefined word directly in the call or a text), `interval(low, high)`.
   - `note(pitch)` (alias note_value) is lib `class Note {key, cents}`: text() ("C4", "A4+20c"), hertz(), octave(),
     name(), plus/minus semitones, equals; play/melody take Notes (hertz_of).
   - A std module's class with methods: insert_module_classes puts the needed classes of used and implicit modules
     in front (units still moves whole, MODULES_COMPILED_AS_PROGRAM) and records functions returning such a class
     (module_returned_classes) so `n = note(C4); n + 7` types. A class with fields only (Sample) keeps sound a used
     module (with_implicit_class_modules). Traps: test a pitch against a class only after si_amount; typed overloads
     can't see a module call's type (`note(midi_hertz(60))`).
   - Still to build: just intonation, velocity, named intervals.
3. **Synthesis** (built: samples/sound/synth.warp, filters.warp; tests/programs/test_sound_synth.rs,
   test_sound_filters.rs, test_sound_aliasing.rs, test_sound_pan.rs). Settings: ADSR `attack`, `decay`, `sustain`,
   `release` (defaults 5 ms / 0 / 1 / 5 ms), `wave_shape = "noise"`, `loudness = -6 dB` (lib decibels), `filter`
   ("lowpass", "highpass", "bandpass": an RBJ biquad per note), `cutoff`, `resonance`, `vibrato` / `vibrato_rate`,
   `tremolo` / `tremolo_rate`, `unison` over `detune`, `pan` (-1 … 1, stereo through sound_samples' channels). Square
   and sawtooth are band-limited (PolyBLEP, lib `step_smoothing`); `wave(shape, p)` without dt stays sharp. Float
   settings are declared `= 0.3 as float`; settings are plain numbers (`cutoff = 1.2kHz` fails, card
   quantity-global). Still to build: wavetables, LFOs and parameter automation.
4. **Signal graph** (built: samples/sound/signal_graph.warp, sound_pipes.warp; tests/programs/test_sound_effects.rs,
   test_sound_pipes.rs).
   - Settings chain: every sound passes distortion → delay → reverb (`distortion`, `delay_time` / `delay_feedback` /
     `delay_mix`, `reverb_mix`), off by default; the settings at the time of `play` are that sound's sends.
   - Pipes: a signal is the list of its amplitudes, a node a function of it: `osc(C4, 1s, "saw") |> lowpass(900) |>
     delay(0.25) |> reverb(0.3) |> gain(0.5) |> out`; nodes osc, lowpass / highpass / bandpass, delay, reverb,
     distort, gain, out; `mix(a, b[, c, d])` sums two to four signals.
   - Shared buses (src/sound/effects.rs, tests/programs/test_sound_buses.rs): `room = bus() |> reverb(0.4) |>
     delay(0.25)`, then `out(signal, room)`, `signal |> out(room)`, `osc(C4, 1s) |> room` (a Bus applied to a signal
     is lib's `call(b: Bus, signal)`, trait Callable) or `play(C4, 1s, bus: room)`. The bus's sounds are mixed, then
     go through its chain once, as a stream, ringing until -60 dB (Chain::rung_on). `room = room |> gain(0)` changes
     the bus from the defining voice's moment: an effect of a name the chain has is replaced in place, Chain::retune
     keeps each stage's state. Bus words `peak`, `low_shelf`, `high_shelf` (RBJ biquads; several peaks at different
     frequencies coexist) and `compress(b, threshold, ratio, attack, release)` (linked peak envelope, no knee, no
     makeup gain).
   - One DSP, in Rust for now (a warp twin later): src/sound/effects.rs `with_effects` runs lib's list effects
     (std_io sound.effect) and the buses natively; in the browser warp.wasm exports web_bus_processed
     (host-compiler.js effectsProcessed, host-sound.js onBus). Probe probes/sound/bus_dsp_in_wasm.mjs.
5. **Samples and files** (built: samples/sound/sound_kick_samples.warp, tests/programs/test_sound_samples.rs): lib
   `class Sample {rate, channels, frames}`; `sample(path)` reads a file through sound.load (src/sound/audio_files.rs:
   WAV in Rust, other formats via afconvert or ffmpeg), sample_seconds, sample_slice, sample_slices, sample_looped,
   play_sample(clip, rate) (rate scales the host sample rate: twice the rate is an octave up), play(x) for a Sample.
   Samples skip the effect chain. The playground decodes WAV only.
6. **Patterns and live loops** (built, plain warp in lib/sound.warp, samples/sound/sound_patterns.warp,
   tests/programs/test_sound_patterns.rs).
   - `pattern "c4 e4 [g4 b4]*2 ~"` is one cycle (a bar, or `for 2 bars`): `~` rest, `[ ]` a group in one step, `*n`,
     `c4,e4,g4` a chord, `c4(3,8)` Euclidean, `<a b c>` one item per cycle in turn, `c4 _ e4` / `c4@2 e4` weights,
     `c4?` / `c4?0.3` chance. Events end at the sample their cumulated seconds reach (no drift). `swing = 0..1`
     delays each second step. The chance is a hash of `pattern_seed`, the cycle and the item's place: the same each
     run, the program's random sequence untouched. A cycle counts how often the voice played that pattern text.
   - `live_loop(drums, 4)` (lib/sound.warp, decision music-plain-details) plays the function `drums` four times in a
     voice of its own named after it (`await drums`); without a count it is endless. Defining a loop again within the
     program is dropped (decision); the playground's live reload below still hands loops over.
   - Playground live reload (tests/web/test_live_handover.rs): while named loops play, Ctrl/Cmd-Enter or editing
     reruns; playground.js handOver keeps the old Worker sounding, the new run's loop of the same name moves onto the
     old loop's grid (host-tasks.js liveCycleStarted) and the old loop is clipped there; a loop the new code drops
     ends after LIVE_RETIRE_SECONDS (4). Unnamed loops are not handed over. Open: a redefined block's variables must
     exist when the voice starts.
7. **MIDI** (src/sound/midi.rs, samples/sound/midi.warp, tests/programs/test_midi.rs).
   - `write_midi(path, notes)` / `read_midi(path)`: Standard MIDI Files (format 0 written at 480 ticks a quarter, 120
     bpm; formats 0 and 1 read, running status, tempo changes) of notes `[start s, pitch, length s, velocity]`, the
     same codec in host-files.js.
   - `midi_on(pitch, velocity = 100, channel = 1)`, `midi_off`, `midi_send(status, d1, d2, port)`,
     `midi_note(pitch, duration, velocity, channel)`. Natively through midir (CoreMIDI, ALSA, WinMM): a device opened
     by name (exact, else the first containing it; "" the first) and kept for the run; headless only `midi out
     <port>: 90 3c 64 at 0.500 s` on stderr. `midi_inputs()`, `midi_outputs()`, `midi_receive(port, seconds)`.
   - A message goes out at its voice's moment (sound.rs moment / on_clock), in the browser with Web MIDI's own
     timestamp. `midi_receive` and the device lists fail loudly in the browser (a Worker cannot wait for the page).
8. **Real-time DSP in warp** (samples/sound/siren_sound.warp, stereo_sound.warp; tests/programs/test_sound_signal.rs,
   tests/web/test_signal_sound.rs).
   - `play(hum, 2s)` of a program function is `play_signal(hum, 2s)` (one parameter `t`, or a pair [left, right]):
     the host calls the export (f64) -> f64 per sample; lib's `signal: (float) -> float` parameter makes the function
     floats to a float (src/lowering/types/function_types.rs, card parameter-typed). On the
     Mixer the samples go out in blocks of 512 queued at most 100 ms ahead, each a sound of the same handle. The call
     blocks; `go { play_signal(…) }` plays it beside the program. DSP state lives in globals declared `as float`.
   - Block-wise: `play_blocks(fill, linear_address(block), 2s)` calls `fill(t)` once a block to fill the program's own
     linear float array (`linear block = float[512]`, notes/linear_arrays.md), `[linear_address(left),
     linear_address(right)]` for stereo; the host reads the cells straight from memory (sound/host_words.rs
     played_blocks), about twice as fast as play_signal.
   - Browser: host-files.js STD_ADAPTERS.sound.signal / blocks compute the samples in the program's Worker, paced
     (a block when the voice holds less than 100 ms) once the page was clicked, else all at once.
   - Still to build: an AudioWorklet running the module, f32 cells, microphone input.

## Our own API or the platform's? (card our-sound)
warp's own small vocabulary on top, Web Audio's model underneath, the platform APIs only as the last stage that moves
samples to the speaker.
- Web Audio's concepts (one clock, a node graph, AudioParams with ramps, AudioBuffers, AudioWorklet) are the common
  ground every modern system converged on, and the playground runs on it anyway. Copy its names where they are good
  (gain, frequency, detune, Q).
- Not its API shape (`ctx.createOscillator()`, `.connect()`, `.start(t)`): warp keeps its words, units (`440Hz`,
  `-6dB`, `1/4 beat`), notes and chords as values, `play … for: …` phrases and `|>` pipes.
- One engine renders the same samples in the browser and natively, so offline rendering (tests) is the same code
  path; only the output differs: cpal (CoreAudio, ALSA/PipeWire, WASAPI) natively, Web Audio's destination in the
  browser. Not a wrapper over each platform's high-level API (AVAudioEngine, GStreamer): those differ in model.
- The WAV-to-afplay path stays the fallback without an audio device.

## Spectrum and the visualizer
`spectrum(samples, seconds, bands)` (lib/sound.warp) gives how loud each band sounds in the `spectrum_window` (512)
samples from `seconds` on, 0 to 1: one Hann-weighted DFT term per band, bands spaced evenly in pitch from 60 Hz to
5 kHz. Plain warp, so the playground computes the same. samples/sound/visualizer.warp paints the bars with falling
peaks each 40 ms, natively a window, headless PNG frames (tests/programs/test_sound_spectrum.rs).
`shown = [0.0, 0.0]` is a list of ints (0.0 is exact): the sample writes `[0.0 as float for …]`.
