# Sound (card basic-sound)

Roadmap card: sound (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

`lib/sound.warp` (sample: samples/music.warp), loaded without `use` when a program calls play, beep, stop_sound or
wait_sound (src/modules.rs SOUND_SETUP_FREE_WORDS). Every other sound word (melody, tone, pattern, live_loop,
render_sound, sound_queued, play_file, the midi words) needs `use sound`; called bare it is the error naming the use
(card bizarre-global, P171; user, 2026-10-11):

```warp
use sound
beep
play 440Hz for: 0.5s
play C4                         // note_seconds (0.5 s)
play [C4 E4 G4] for: 1s          // a chord
melody [C4 D4 E4]               // one after another
melody [G4 E4 C4] each: 200ms    // or melody([G4 E4 C4], 200ms)
tone(220Hz, 300ms, "square")    // sine, square, triangle, sawtooth
tone 330Hz for 0.1s
play note("F#4") for: 250ms      // note(name) needs `use sound` when called alone
```

- The samples are made in warp (22050 Hz, 16-bit mono, 5 ms fade at both ends) and cross to the host word
  `sound_samples(samples, count, rate)` as whole numbers ≥ 0 (amplitude + 32768), read in one call by list_to_ints.
- Natively (src/sound/mod.rs) a WAV in `$TMPDIR/warp-sound/` (sound-<pid>.wav, sound-<pid>-2.wav …: one name per process, deleted once played; card sound-wavs) played by afplay/paplay/aplay to its end; under tests, CI and
  WARP_NO_WINDOW only the file and a `sound 0.50 s: <path>` line on stderr: tests stay silent.
- Browser: the worker posts `{type: "sound"}`, the page (playground.js playSound) plays it with WebAudio, queued one
  after another; a new run silences what still plays (typing reruns the code). A browser starts audio only after a
  gesture: the run on page load stays silent, ▶ plays. A run without page hooks (tests) is silent.
- Music files (card sound-library): `play "song.mp3"` / `play ./mozart.mp3` / `play_file(path)` play a wav, mp3, ogg, flac, aiff or m4a in
  the background (format by its first bytes; card play-mozart: `./` or `../` glued to the rest of an operand is a path
  literal, the text `"./mozart.mp3"`, src/warp_parser lookahead.rs starts_path_literal; `xs ./ 2` still divides), `stop_sound` stops them. Natively (src/sound/mod.rs play_file) the first
  player the system has for the format: afplay (no ogg), paplay (no mp3), ffplay, mpv; headless only the check and a
  `sound file <format>: <path>` line (tests/programs/test_sound_files.rs). In the playground an `<audio>` by its URL
  (host-files.js STD_ADAPTERS.sound → worker → playground.js playSoundFile); a new run stops it. Card sound-file-failure: the
  file is decoded first by ffprobe or afinfo (a broken one fails play, also headless); a player failing within 0.3 s
  fails play, a later failure prints `sound file <path>: <player> failed (<status>): <its error output>`; without a
  decoder on the machine only the format is checked. In the browser a failure reaches the console.
- Units: `Hz` is `1/s`, `kHz` `1/ms` (src/numbers/units.rs UNIT_ALIASES); `si_amount(x)` takes a quantity or a plain number.

A module's `global` is a setting: the program's top-level `note_seconds = 0.25` or `tempo = 90` sets it (modules.rs
shadowed_apart leaves globals alone; other program variables named like a module word still shadow it). Before
2026-10-10 such an assignment made a program variable of its own and the module silently kept its value.

## Spectrum and the visualizer (card winamp-like)
`spectrum(samples, seconds, bands)` (lib/sound.warp) gives how loud each of `bands` frequency bands sounds in the
`spectrum_window` (512) samples from `seconds` on, 0 to 1: one Hann-weighted DFT term per band (`band_level`), bands
spaced evenly in pitch from `band_low` 60 Hz to `band_high` 5 kHz (`band_frequency`, `nearest_band`). Plain warp, so the
playground computes the same; 16 bands take about 11 ms natively. A single tone at loudness 0.3 reads about 0.16 in its
band. samples/visualizer.warp plays a tune and paints the bars with falling peaks each 40 ms of real time (clock),
natively a window, headless PNG frames (tests/programs/test_sound_spectrum.rs).
Bugs it found, fixed on the way: float elements of a comprehension or a function's list read as ints in loop bodies, as
arguments of a parameter that also gets ints, and in `==` ("not an int", tests/lists/test_number_list_elements.rs).
`shown = [0.0, 0.0]` is still a list of ints (0.0 is exact), so the sample writes `[0.0 as float for …]` until the
exact-decimal list decision (warp-numbers) lands.

## MIDI (card midi-out, layer 7 started; samples/midi.warp, tests/programs/test_midi.rs)
`write_midi(path, notes)` / `read_midi(path)`: Standard MIDI Files (format 0, 480 ticks a quarter at 120 bpm; reading takes
formats 0 and 1, running status, tempo changes) of notes `[start s, pitch, length s, velocity]`; they cross to the host as
flat whole numbers `[start ms, key, length ms, velocity …]` (src/sound/midi.rs natively, host-files.js midiFileOf / midiNotesOf in
the browser, the same codec twice). `midi_on(pitch, velocity = 100, channel = 1)`, `midi_off`, `midi_send(status, d1, d2, port)`.
Natively messages go through midir (CoreMIDI, ALSA, WinMM; card midi-receive): an output or input is opened by name
(exactly, else the first containing it; "" the first device) and kept open for the run; headless only
`midi out <port>: 90 3c 64` on stderr. `midi_inputs()`, `midi_outputs()` list the devices (ø without any).
`midi_receive(port = "", seconds = 0)` gives the next message [status, data1, data2], waiting for it (seconds > 0: at
most that long, then ø); messages between two calls queue up. tests/programs/test_midi.rs plays a CoreMIDI virtual
source into a program (macOS). Browser: Web MIDI on the page (playground.js sendMidi, first output or by name, failures
to the console); `midi_receive`, `midi_inputs`, `midi_outputs` fail loudly there (a Worker cannot wait for the page).
Clock-timed built (card midi-clock): a message goes out at its voice's moment (sound.rs moment / on_clock): after the
sounds queued before it, at an `at`'s time, in a task on its own voice; natively a thread sleeping until then (wait_sound
and the process's end wait for it, stop_sound drops it), headless the line names the place (`midi out : 90 40 64 at 0.500 s`),
in the browser Web MIDI's own timestamp (host-sound.js voiceSounded(0).at → playground.js output.send(message, time)).
`midi_note(pitch, duration, velocity, channel)` is on, the voice resting the duration (std_io sound.rest: the voice goes on
without a sound), off. A task Worker in the browser posts its sounds and MIDI to the page itself (host-tasks.js
addPageSoundWords, card playground-live): relayed through the program's Worker, which waits for the task in
Atomics.wait, they arrived bunched once main returned; handles come from one counter the run's Workers share.
A loop variable named `note` inside lib/sound.warp breaks: `note` is the module's own word ("not a list").

## Probes and quirks (from the warp-sound handover, 2026-10-10)
- probes/sound/sound_words.warp exercises the sound words; probes/music_go/ the `play`/`melody` phrases;
  probes/music_outlives_file.py that the player stops with warp.
- Run sound probes under WARP_NO_WINDOW=1: otherwise sound plays on the user's Mac. Headless, sound only prints
  `sound N s: <wav path>` on stderr.

## What professionals expect (user question 2026-10-09; roadmap, nothing of it built yet)

The words above are layer 1, the toy layer. Each layer below keeps the ones above working and lowers to them.

1. Non-blocking, clocked playback. Built (step 1, 2026-10-10): `play` returns at once, its sound queued on the audio
   clock behind the sounds before it (natively a player thread, src/sound/mod.rs queued; the playground's WebAudio queue);
   `sound_queued()` the seconds still to sound, `wait_sound`, `stop_sound` drops the queue, the process's end waits.
   Its headless timing test measures around `wait_sound` inside the Warp program and uses a low sample rate for a
   30-second audio-clock duration. The clock case stays long without synthesis or child compilation dominating the
   assertion under full-suite load (card headless-wait).
   It also waits for music files playing (`play "song.mp3"`, card make-background, 2026-10-10): before, warp ended and
   left the player (afplay) playing in the background, out of reach of ctrl-c and of Sublime's cancel (SIGTERM to the
   build's process group); now both stop warp and the player together (probes/music_outlives_file.py, silent).
   Offline render built (step 2, 2026-10-10, from layer 5): `render_sound("song.wav")` writes the sounds since the
   last render (or the start) one after another into one WAV and gives its seconds; headless too, so tests check audio
   without speakers (tests/programs/test_sound_render.rs). Since card save-sound (user 2026-10-15) it is written
   `save sound as: "song.wav"` / `save(sound, "song.wav")`: save overloaded by type, the word sound as save's argument a
   lib/sound.warp Recording (music_words.rs with_recording); render_sound stays as the old name with a note. The samples are kept in memory (44 KB a second at 22050 Hz),
   not read back from $TMPDIR/warp-sound: that folder is shared by all warp processes. The playground refuses it (its
   files are texts).
   Handles built (step 3, 2026-10-10): `play`, `melody`, `tone` and `play "song.mp3"` give the sound's handle, its
   number in the run (sounds and music files counted together, from 1; the playground counts per run in worker.js);
   `stop_sound(h)` drops that one from the queue or stops its player, `stop_sound` (handle 0) all of them
   (tests/programs/test_sound_handles.rs). lib/sound.warp's sound names the handle before returning it: card
   block-data-sound.
   Tempo built (step 4, 2026-10-10, samples/rhythm.warp): `1/4 beat`, `2 beats`, `1 bar` are seconds at the
   module's `tempo` (beats per minute, 120) and `beats_per_bar` (4), anywhere in a program that uses sound: the
   amount is the item before the word, of an assignment its value (src/lowering/words/music_words.rs → lib beat_seconds,
   bar_seconds; tests/programs/test_sound_tempo.rs). Not units of the static unit checker: a beat is no SI time.
   Voices built (step 5, 2026-10-10, samples/voices.warp): every task (`go { }`) is a voice. It forks its starter's
   place in time (src/sound/mod.rs Voice, joined in src/host/tasks.rs spawn), so a task's notes sound along with the
   starter's instead of after them; each queued sound plays from its own sleeping thread (overlapping players, the OS
   mixes), and render_sound mixes the sounds at their logical places (tests/programs/test_sound_voices.rs).
   In the Playground too (card playground-go): each Worker keeps its voice's end on a clock the page and the Workers
   share (host-tasks.js voiceSounded, performance.timeOrigin + now), a task Worker starts from its starter's, and the
   page plays each sound at its `at` (playground.js playSound). A task's sounds reach the page only once the program's
   Worker is back in its event loop (after `await`), so a start already past plays at once. A task's sounds go back
   with its result to the starter's run (host-tasks.js withTaskSounds), each at its place on its voice
   (voiceSounded), and render_sound mixes them as natively (host-files.js mixed; card playground-drops).
   Scheduling built (card sample-accurate, samples/arrangement.warp): `at 2 beats play C4`, `at 1 bar { … }` is the
   statement between sound_at(time) and sound_at_end() (music_words.rs): its sounds start at that time counted from
   the voice's first sound (Voice origin), then the voice goes on where it stood. Exact to the sample in render_sound,
   which now counts from where the last render ended (so the silence before an `at` stays) and in the browser's
   WebAudio start(at); native live playback only to the millisecond (a sleeping thread per sound, then afplay).
   Ramping a handle's gain is fade_sound (card sound-gain-ramps, "Native output built" below).
2. Note names built (2026-10-10): parsed, not a table: a letter A–G, `#`/`♯` or `b`/`♭`, an octave 0–9 is its
   equal-tempered frequency to a hundredth of a Hz (A4 = 440), when the program uses sound and doesn't define that
   name; `F#4` parses as `F # 4` and counts as the note unless the program defines `F` (music_words.rs; the parser
   lets ♯ ♭ continue a name). tests/programs/test_sound_notes.rs.
   Music values built (card music-values, samples/harmony.warp, tests/programs/test_music_values.rs): a note name
   lowers to `midi_hertz(midi)`, so `tuning = 442` (A4, default 440) tunes every note; `midi(pitch)`,
   `note_name(pitch)` (sharps: "D#4"); `transpose(pitch or list, semitones)` keeps a note a note (transpose(C4, 12)
   == C5 to the hundredth) and multiplies any other frequency; `chord(root, kind)`, `scale(root, kind)` are lists of
   notes, kinds in lib chord_intervals / scale_intervals, written as a word the program doesn't define (only directly
   in the call: `f(major)` passes an undefined symbol) or as a text. Notes stay frequencies at run time; sound's
   transpose/2 and matrix's transpose/1 coexist.
   Note values built (card note-value, tests/programs/test_note_values.rs): `note(pitch)` (alias note_value) is lib
   `class Note {key, cents}` with text() (prints "C4", "A4+20c"), hertz(), octave(), name(), plus/minus semitones,
   equals; `note("C#4")`: music_words lowers a note-name text in a call of note like the bare name; `interval(low,
   high)` is semitones; play/melody take Notes (hertz_of). Bare names stay frequencies (user decision 2026-10-14: the
   word note, `C4` stays 261.63; note(text) gave the frequency before, now note_hertz). Lib trap: test a pitch against
   a class only after si_amount (a quantity `is Note` fails statically, card quantity-is-class), and typed overloads
   can't see a module call's type (`note(midi_hertz(60))` fits no variant). How a std module's class with methods
   works for a module that stays a used module: insert_module_classes puts the needed classes of used AND implicit
   modules in front (units excepted: it still moves whole, MODULES_COMPILED_AS_PROGRAM, its methods need its number
   words as program code), and records the module functions returning such a class (module_returned_classes) so
   class_methods types `n = note(C4)` for `n + 7`. Still to build: just intonation, velocity, named intervals.
3. Built (2026-10-10, samples/synth.warp, tests/programs/test_sound_synth.rs): the ADSR envelope of every note,
   module settings `attack`, `decay` (seconds), `sustain` (a share), `release` (seconds), defaults 5 ms / 0 / 1 / 5 ms
   = the old click-free fade; `wave_shape = "noise"`; gain in decibels, `loudness = -6 dB` (`-6dB`, `x = -6 dB`;
   music_words.rs → lib decibels). The float settings are declared `= 0.3 as float` (card float-global: a float
   assigned to a global declared with a decimal fails "not an int" in the functions that read it).
   Built (card synth-filters, samples/filters.warp, tests/programs/test_sound_filters.rs): settings `filter`
   ("lowpass", "highpass", "bandpass": an RBJ-cookbook biquad over each note's amplitudes), `cutoff` (Hz),
   `resonance` (Q, 0.707 flat), `vibrato` (semitones) at `vibrato_rate`, `tremolo` (a share) at `tremolo_rate`,
   `unison` voices spread over `detune` semitones. Plain numbers: `cutoff = 1.2kHz` fails (card quantity-global).
   The filter runs per note, so its state starts fresh at every note.
   Band-limited (card reason-beep, tests/programs/test_sound_aliasing.rs, probes/sound/beep_spectrum.py): square and
   sawtooth jumps are smoothed over a sample (PolyBLEP, lib `step_smoothing`). The naive ones folded their harmonics
   above 11025 Hz back as buzz: beep (880 Hz square) had 3.9 % of its energy off its harmonics, a sawtooth 5.4 %, now
   0.7 % each (a sine: 0.6 %, its fades). `wave(shape, p)` without dt stays sharp.
   Stereo built (card stereo-pan, tests/programs/test_sound_pan.rs, tests/web/test_rendered_pan.rs): `pan` (-1 left,
   1 right, 0 mono) makes lib panned interleave left, right, the far side scaled by 1 - |pan|; sound_samples carries
   the channels (src/host.rs, 4th parameter), the WAV/AudioBuffer has them, render_sound mixes in stereo when any
   sound is (mono on both sides alike), natively and in host-files.js mixed. Voice stealing built (card
   sound-voice-stealing, tests/programs/test_sound_engine.rs): at most 32 sounds sound at once (sound_engine.rs
   MAX_VOICES, playground.js stealFor); one more steals the oldest sounding then, which fades out in 5 ms and leaves.
   Live only: render_sound mixes every sound (a render has no device to keep up with). Still to build:
   Synthesis. Oscillators (sine, saw, square, triangle, noise, wavetable, band-limited, detune, unison), ADSR
   envelopes, filters (lowpass/highpass/bandpass with resonance), LFOs and parameter automation (ramps; card sound-gain-ramps), gain in dB,
   stereo pan, polyphony with voice stealing.
4. A signal graph. `osc(220Hz, saw) |> lowpass(1.2kHz, q: 4) |> delay(3/8 beat) |> reverb(0.3) |> gain(-6dB) |> out`:
   nodes and buses as values, effects (delay, reverb, compressor, distortion, EQ), a mixer.
5. Samples and files. Load WAV/MP3/OGG/FLAC, play at a pitch or rate, slice, loop points (card sound-samples); render offline to a file
   (`render 8 bars to "song.wav"`), which is also how tests check audio without speakers.
   Samples built (card sound-samples, samples/sound_samples.warp, tests/programs/test_sound_samples.rs): lib/sound.warp
   `class Sample {rate, channels, frames}`; `sample(path)` reads a file through the host word sound.load
   (src/sound/audio_files.rs: WAV decoded in Rust, other formats converted by afconvert or ffmpeg first), then
   sample_seconds, sample_slice(clip, from, to), sample_slices(clip, n), sample_looped(clip, seconds, from, to),
   play_sample(clip, rate) and play(x) for `x is Sample`. The rate trick: play_sample hands the frames on with the host
   sample rate scaled by rate (twice the rate = twice as fast, an octave up), and render_sound mixes all sounds at the
   highest rate among them (at_rate, natively and in host-sound.js). The playground decodes WAV only (wavFrames).
   Samples skip the effect chain: they sound as recorded. Trap: a lib parameter named `s` (a unit) shadows `0.5s`
   program-wide, hence `clip`. A class with fields only (Sample) keeps sound a used module: with_implicit_class_modules
   moves only modules whose classes have methods (units' Quantity), else note names stop lowering.
6. Patterns and sequencing. Sonic Pi `live_loop`, Tidal mini-notation (`"c4 e4 [g4 b4]*2"`), Euclidean rhythms,
   swing, randomness with seeds; live coding: redefine a loop while it plays.
7. MIDI and devices. MIDI in/out (CoreMIDI, Web MIDI), choosing the output device, channels, buffer size and latency.
8. Real-time DSP in warp itself. A user-written `process(block)` compiled to wasm runs in an AudioWorklet in the
   browser and in a CoreAudio render callback natively: the same code both ways, which is where wasm-first warp beats
   other languages. Needs f32 arrays without boxing (today samples cross as a list of Ints), no allocation in the
   callback, 44.1/48 kHz stereo.
   Native first step built (card real-time, samples/siren_sound.warp, tests/programs/test_sound_signal.rs):
   `play_signal(hum, 2s)` calls the program's `hum(t)` for every sample (t in seconds, at sample_rate), mono. The
   host word (host/native.rs played_signal) calls the export typed (f64) -> f64; music_words.rs makes the function
   of floats to a float (untyped parameters `: float`, body `as float`, so `hum(t) := 3.0` works too). On warp's own
   output (sound_engine) the samples go out in blocks of 512 queued at most 100 ms ahead of the speaker
   (sound.rs play_signal), each block a sound of the same handle on the Mixer, so stop_sound(handle) ends it; the
   audio callback itself only mixes (no wasm, no allocation). Headless, or on afplay, it is one sound: render_sound
   and the tests read the same samples. The call blocks the program while it plays; `go { play_signal(…) }` plays it
   beside it. State between samples lives in globals declared `as float`: an exact decimal global
   (`global level = 0.0`) is an exact ratio whose denominator gains about a bit a sample (the blow-up beyond that,
   unreduced ratios, is fixed: card exact-decimals), so floats stay the way for DSP state.
   About 10× faster than real time in a debug build (20 s of two sines in 2.3 s, compile included).
   Block-wise built (samples/stereo_sound.warp, same test file): `play_blocks(fill, 2s)` calls `fill(block, t)` once
   a block, block a linear float array (notes/linear_arrays.md) of 512 cells to write, t its first sample's time.
   music_words.rs with_block_signals declares `linear fill·block = float[512]` and `fill·blocks(t: float)`, which
   calls fill with it (so fill's parameter is the linear array), and writes the call out as std_io("sound", "blocks",
   [fill·blocks, linear_address(fill·block), …]); the host (native.rs played_blocks) calls fill·blocks and reads the
   cells straight from memory. 20 s of a sine: 2.0 s against play_signal's 4.6 s (debug, compile included). Cells are
   f64 (linear arrays have no f32 yet); the host takes them as they are.
   Stereo built (undoable default): `play_signal([left, right], 2s)` calls left(t) then right(t) for each sample;
   `play_blocks(fill, 2s)` with fill of three parameters `(left, right, t)` gets two blocks (fill·block, fill·right).
   The host takes one value or a pair (native.rs channels_of) and interleaves the frames (sound.rs play_blocks).
   `play hum for: 2s` (user, 2026-10-12: no snake case; `signal` is a reactive keyword, so no synonym word): play of
   one of the program's functions is play_signal of it (one parameter, or a pair [left, right] of such), play_blocks
   (two or three parameters); music_words.rs with_played_functions. play_signal and play_blocks stay the explicit forms.
   Browser first step built (tests/web/test_signal_sound.rs): host-files.js STD_ADAPTERS.sound.signal / blocks compute
   all samples at once in the program's Worker (the function as closure_apply's callable, blocks read from memory)
   and play them as one sound (host.js holder.sounded), as natively headless. Paced once the page was clicked or
   typed in (window_open, so audible): the Worker computes a block when the voice holds less than 100 ms
   (playedSignal, host-tasks.js soundsQueued), each block a sound of the signal's handle (stop_sound ends them all),
   the wait a sleep without runaway time. Unclicked (CI) everything at once, one sound.
   Real-time playback (card real-time-playback, the user's `loop { go {melody…}; melody…; tempo *= 1.05 }`, kept in
   probes/sound_experiments/voices.warp): heard, a voice's next sound waits until its queued sounds end within
   PLAY_AHEAD (0.5 s; sound.rs paced, host-files.js holder.sounded), except in an `at`, which places its sounds itself.
   Before, the loop computed minutes ahead, a new task thread a round, its exact tempo growing digits a round (card
   exact-tempo-growth; the tempo is a float now, lib/sound.warp). In the browser a task's sounds still reach the page only when the program's Worker is back in
   its event loop, so a paced endless loop with tasks hears the main voice only (a MessagePort per task Worker to the
   page would fix it). Measured (release, probes/sound/voices_loop_timing.warp): a round computes in 15-60 ms for
   0.4-2.2 s of sound, so pacing, not speed, was missing.
   Still to build: an AudioWorklet running the module (glitch-free under a busy Worker), input (a microphone as the
   signal's argument).

Order of value: 1 + 5 (handles, clock, offline render) first, they change the architecture; then 2, 3, 4; 8 last.

## Patterns and live loops (card patterns-live, samples/patterns.warp, tests/programs/test_sound_patterns.rs)
Layer 6 built on the beat clock, in plain warp (lib/sound.warp), so the playground runs it too:
`pattern "c4 e4 [g4 b4]*2 ~"` is one cycle (a bar, or `for 2 bars`) of steps sharing the cycle's seconds: `~` rest, `[ ]` a group
in one step, `*n` n times in its step, `c4,e4,g4` a chord, `c4(3,8)` Euclidean (step i sounds when i·3 mod 8 < 3). The pattern is
parsed (parse_pattern), laid out to [frequencies, seconds] events (pattern_events) and made one sound whose events end at the sample
their cumulated seconds reach (no drift), so the voice goes on after the cycle; `swing = 0..1` (module setting) delays each second
step of a group by swing·step/3 (1: triplet feel). `live_loop 4 { … }` is `live_voice = go { for … 0..4 { … } }`
(music_words.rs lower_live_loops, before go_blocks): a voice of its own on the beat clock, so loops and `play` overlap; `await live_voice`
waits for the last loop (renders after live loops need it: the task adds its sounds when it runs).
Built (card pattern-extras): `<a b c>` one item per cycle in turn (a nested `<>` turns on its own visits: `<c4 <~ e4>>` is
c4 ~ c4 e4), `c4 _ e4` / `c4@2 e4` weights (c4 two steps of three), `c4?` / `c4?0.3` sounds in that share of cycles. A cycle is
how often the voice played the same pattern text before (lib pattern_cycle, the texts kept in `patterns_played`; a task
counts its own). The chance is a hash of `pattern_seed` (setting, 0), the cycle and the item's place, not random(): the same
each run and each render, and the program's own random sequence stays untouched. `live_loop drums 4 { … }` names the voice
(`await drums`), the unnamed form keeps `live_voice`.
Built (card live-redefine, tests/programs/test_live_redefine.rs): a named loop defined again in the same block
(`at 2 bars live_loop bass { … }`) is no new voice: music_words with_live_loops makes the first definition a voice
choosing each cycle among all the blocks, the later ones set `bass_since#k = sound_moment()` (a shared float array,
-1 unset). sound_moment (host word sound.moment, natively seconds of the voice's moment on RUN_CLOCK, in the
playground voiceMoment on the Workers' shared clock) is where the voice's next sound would start; a cycle plays the
latest block whose moment it reached, 50 ms early counting (voices start microseconds apart). The first definition's
count stays; `live_loop name { … }` without one is endless (heard voices pace 0.5 s ahead, silent runs don't).
Built (card playground-live, tests/web/test_live_handover.rs): playground live reload. A named loop's block starts
with `live_cycle_started("drums")` (music_words LiveLoop::announced; natively a no-op). While named loops play,
Ctrl/Cmd-Enter or editing reruns (the Run/Stop button still stops): playground.js handOver retires the old Worker
(it keeps sounding) and starts the new run with liveHandover; the new run's loop of the same name moves its first
cycle onto the old loop's grid ({start, period}, host-tasks.js liveCycleStarted), and endHandedOver clips the old
loop's sounds at that cycle. Once all its loops are taken the old Worker is terminated; a loop the new code drops
ends at its first cycle after LIVE_RETIRE_SECONDS (4) (Sonic Pi keeps it playing). Unnamed loops are not handed
over; Stop terminates every retired Worker too. Still open: a redefined block's variables must exist when the voice
starts.

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
  `play … for: …` phrases and `|>` pipes. Those lower to the graph; a program never sees a context object.
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

## Signal graph built (card signal-graph, 2026-10-10, samples/signal_graph.warp, tests/programs/test_sound_effects.rs)
The simple form of layer 4: every sound passes a fixed effect chain distortion → delay → reverb between the voices and the
output (lib/sound.warp `through_effects`, run in `sound()` over the whole phrase, so a melody has one tail). Module settings
`distortion` (soft-clipper drive), `delay_time` / `delay_feedback` / `delay_mix`, `reverb_mix` (four Schroeder combs, 0.6 s
tail); all off by default (no cost). The settings at the time of `play` are that sound's sends, so each voice (task) sets its
own. Pure warp, so the playground runs the same. The pipe form is built (card sound-pipe-graph,
samples/sound_pipes.warp, tests/programs/test_sound_pipes.rs): a signal is the list of its amplitudes, a node a function
of it, so `osc(C4, 1s, "saw") |> lowpass(900) |> delay(0.25) |> reverb(0.3) |> gain(0.5) |> out`; nodes osc, lowpass /
highpass / bandpass (frequency, q), delay (seconds, feedback, mix), reverb (mix), distort (drive), gain, out (plays it
through `sound`, so the settings chain and pan still apply); `mix(a, b[, c, d])` sums signals to the longest, a bus as a
value. The settings chain and the nodes share biquad / echoed / roomed / clipped. Speed: a mix over a list of signals in
a loop was quadratic (card list-call-copy), so mix takes two to four signals.

## Shared buses, stage 1 (card shared-buses, 2026-10-15, src/sound/effects.rs, tests/programs/test_sound_buses.rs)
`room = bus() |> reverb(0.4) |> delay(0.25)`, then `out(signal, room)` or `signal |> out(room)` from the program or
any task: the bus's sounds are mixed, then go through its chain together, once, as a stream, ringing on until -60 dB
after the last (Chain::rung_out). Natively the Mixer runs each bus in its device callback (fill); render_sound mixes
each bus apart and runs the same Chain (effects::processed). lib: class Bus {id, chain}, the effect nodes on a Bus append
to its chain (host word sound.bus), `out(signal, b: Bus)` sends (sound.send, a thread's current bus around its sound).
The DSP is the lib formulas in Rust, streaming. Plan (supervisor, user asked whether Rust or warp holds the one DSP):
stage 2 the browser runs the same code (warp.wasm export for render_sound, an AudioWorklet live; until then the
playground plays bus sounds dry and says so), stage 3 a bus's effects changed while it plays, stage 4 compressor, EQ.

Stage 3 built (2026-10-15): `room = room |> gain(0)` changes the bus from the defining voice's moment (where its queued
sounds end). An effect of a name the chain has replaces it in place (with_effect), else it appends. Chain::retune keeps
each stage of the same kind with its state (comb echoes, biquad history), so a delay set again rings on; another kind
starts afresh. Live: Mixer::change_bus(bus, frame, effects) stores pending changes per Bus, fill splits the bus's buffer
at their frames. render_sound: BUSES keeps every definition at its place since the last render, effects::processed
retunes at their frames, the last definition stays for the next render. `out(signal, bus)` restores the bus the thread
sent to before (sound.send returns it, natively and in host-sound.js).

Stage 4 built (2026-10-15): bus words `peak(b, frequency, db, q = 1)`, `low_shelf`/`high_shelf(b, frequency, db,
q = 0.707)` (RBJ cookbook biquads, FilterKind::Peak/LowShelf/HighShelf carry the dB) and `compress(b, threshold = -18,
ratio = 4, attack = 0.01, release = 0.1)`: one peak envelope over the loudest channel (linked, the stereo image stays), the
dB over the threshold scaled by 1 - 1/ratio, no knee, no makeup gain (add `gain`). EQ bands are several of a name
(card bus-peak): `peak(200Hz, 3) |> peak(3kHz, -2)` keeps both, one at a band's frequency replaces it. Bus-only for now: no list-signal versions in
lib (they wait for the one-DSP answer, Rust or warp).

## Native output built (card native-sound, 2026-10-12, src/sound/sound_engine.rs, tests/programs/test_sound_engine.rs)
On macOS every sound now goes into one cpal output stream (CoreAudio) instead of one afplay process per WAV. A `Mixer`
holds the sounds, each converted to the device's rate (linear interpolation) and channels, starting at its frame;
the device callback calls `fill`, which mixes them, advances the frame and drops finished ones. `sound.rs scheduled()`
gives an Instant; the frame is (instant + 60 ms lead − stream start) × rate, so back-to-back sounds stay contiguous
to the sample and `at 2 beats` lands to the sample live, not only in render_sound. The callback never waits:
`try_lock`, silence when a queueing sound holds the lock. `fade_sound(handle, gain, seconds)` (lib/sound.warp) ramps a
playing sound's gain linearly, frame by frame, 0 = all handles. Fallbacks: no output device → afplay as before
(fade_sound then fails loudly); not audible (WARP_NO_WINDOW, CI) → nothing plays, the WAVs are still collected for
render_sound. fade_sound everywhere (card sound-gain-ramps, tests/programs/test_sound_fades.rs,
tests/web/test_rendered_fades.rs): the browser ramps a GainNode per sound (playground.js faded), and render_sound
applies the same ramps (sound.rs faded_gain, host-files.js fadedGain): each fade starts where the speaker stood on the
caller's voice (its place minus what is still queued) and ramps from the gain it finds there. Not yet: Linux/Windows
output (cpal supports them; only the macOS dependency is declared), LFOs (a gain following a function of the time),
and DSP in the callback itself (card real-time builds on this Mixer).
