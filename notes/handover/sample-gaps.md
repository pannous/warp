# Handover: sample-gaps (warp-samples), 2026-10-27

## Landed today
- data blocks / data parameters (3ff08b4504); class operators outside the class (dbcca67aa3)
- π, τ as an exact Real in lib/reals.warp, an implicit type module (dc04aa0555)
- units as values in lib/units.warp (a24528b898): 5km = 5 * km, dimension classes, == by amount, sleep(duration: Time);
  each unit a row of unit_table (9ac5173269), so a program's own s or m leaves them alone
- a method's name changes no syntax (in(u) keeps for-in, float() keeps is float); {"K": K} keeps the value (b1a4e2976c)
- notes/sample_status.md: 80/224 samples run (main 45a41fcd2d)

## Open, in order
1. paint: DONE 4431a21768 (host words shadowed per arity; paint_function, paint_text, window_size run)
2. sound + units: DONE 45de9529de (lib-sound, missing-pure, class-position-index): wasm memory bodies, class samples
   in linear memory, a class's position/index/count, F#4; hum_blocks runs. Still failing in samples/sound: play(C4, d)
   (card sound-hertz-length), unknown module names (play, at, unison, tuning …: sound's slices not reached), traps in
   rhythm/synth/voices, sound_pipes variants; hum(t: float) := 0.3 * t has no float result (card float-result-hum)
3. `sleep 50ms` without parentheses misses the expected-type units (told warp-text-field; snake, game_of_life)
4. ∞ and ℯ can't be defined as functions, big floats give 0 (card float-overflow-inf); Real where a float is wanted
   (real-for-float DONE bee9079dc5), `π is real` (pi-real-number)
Parked: `on` handlers (user discussing), stored (handler shape), foreign spellings (user audit).

Sweep script: the session scratchpad sweep.sh (each sample with warp pure, 30 s, one at a time); logs in
~/Library/Logs/warp/samples/sweep.txt.
