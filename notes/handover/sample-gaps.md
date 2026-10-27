# Handover: sample-gaps (warp-samples), 2026-10-27

## Landed today
- data blocks / data parameters (3ff08b4504); class operators outside the class (dbcca67aa3)
- π, τ as an exact Real in lib/reals.warp, an implicit type module (dc04aa0555)
- units as values in lib/units.warp (a24528b898): 5km = 5 * km, dimension classes, == by amount, sleep(duration: Time);
  each unit a row of unit_table (9ac5173269), so a program's own s or m leaves them alone
- a method's name changes no syntax (in(u) keeps for-in, float() keeps is float); {"K": K} keeps the value (b1a4e2976c)
- notes/sample_status.md: 80/224 samples run (main 45a41fcd2d)

## Open, in order
1. paint (6 samples, card paint-samples-branch): branch pure-paint has the prelude overload
   `paint(picture, width, height)`. Blocked in core: emitter.warp checks is_program_call by name before module.host, so the
   prelude's 3-parameter paint hides the host's 4-argument paint (pixels, w, h, shader values) and its own inner call is
   refused. Next step: let a call whose arity no program function takes fall through to the host word (emitter.warp
   around line 782, is_program_call), a small core change: smoke + self-check + gate.
2. sound + units (card lib-sound): si_amount should read a Quantity; `use units` + `use reals` in lib/sound.warp fails
   the sound samples on linear_new(…), floor of a ? (/ through the class operator dispatcher), distortion, a list for a text
3. `sleep 50ms` without parentheses misses the expected-type units (told warp-text-field; snake, game_of_life)
4. ∞ and ℯ can't be defined as functions, big floats give 0 (card float-overflow-inf); Real where a float is wanted
   (card real-for-float), `π is real` (pi-real-number)
Parked: `on` handlers (user discussing), stored (handler shape), foreign spellings (user audit).

Sweep script: the session scratchpad sweep.sh (each sample with warp pure, 30 s, one at a time); logs in
~/Library/Logs/warp/samples/sweep.txt.
