# Sound (card basic-sound)

`lib/sound.warp` (sample: samples/music.warp), loaded without `use` when a program calls play, melody, tone or beep (src/modules.rs SOUND_WORDS):

```warp
beep
play 440Hz for 0.5s
play C4                         // note_seconds (0.5 s)
play [C4 E4 G4] for 1s          // a chord
melody [C4 D4 E4]               // one after another
melody([G4 E4 C4], 200ms)       // each 200 ms
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
- Units: `Hz` is `1/s`, `kHz` `1/ms` (src/units.rs UNIT_ALIASES); `si_amount(x)` takes a quantity or a plain number.

Limits found on the way: a phrase with the word `each` loses its unit argument (card phrase-each), so the per-note
duration of melody is a plain call; assignments to the module's globals (`note_seconds = 0.25`) from the program do not
reach the module.
