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
   rhythm/synth/voices, sound_pipes variants. Since: play(C4, d) cbb5e3a005 (rhythm, synth, voices run), signals of
   any number and function-arity overloads a2cdbb6e5c, global function values 3375c04b54. Left in samples/sound: module
   names not reached (play, at, unison, tuning, distortion, pattern, midi_on), help(), phase(…), sound_pipes, `on`
3. `sleep 50ms` without parentheses misses the expected-type units (told warp-text-field; snake, game_of_life)
4. ∞ and ℯ can't be defined as functions, big floats give 0 (card float-overflow-inf); Real where a float is wanted
   (real-for-float DONE bee9079dc5), `π is real` (pi-real-number)
5. 2026-10-28 night: bare-word 70b2333892 (Chord/Scale enums), block-parameter-refusal 9df63cdf18 (`body!`, `at
   bars(0) {…}`), module-global-retune → charged values as the lib/sugar/heads.warp rule `where $h is varying`
   9201044224, sound-effect-error 551c7d17fd (harness: host lists in one go, gc_values::vector_list), play-mozart
   516ec2eeaa (play/beep/stop_sound/wait_sound bare). Open:
   - global-words-declared (pushed branch be9ffab461): the core table becomes the marker `global play(tone) := …` in
     the module (interviewer's call); lib/file.warp's write/exists/append_file marks wait for a seed refresh (the seed
     compiles compiler.warp, which reaches file.warp, and has no stripping rule yet)
   - main's sound regression since ~06:25: `use sound; si_amount(5ms)` → "floor of a ? at 5:26"; not the harness, nor
     any cache tried (notes in the Supervisor thread); cause unknown
   - p71-says (assigning a `:=` name is an error; test_charged_values:6 expects it), visualizer-pure-refused
6. 2026-10-28 morning: pure-compiled 30486e5e19 (harness: `warp pure` links C imports from warp.c_functions like
   warp-host, one linker in warp_runtime::c_imports; test/ffi 110→87 failing, the rest compile-side), letters-letters
   c9382ea3f5 (c++ steps a character's code point), foo-of-any 10ade6cb89 (`square of number = it*it`; `of` binds
   tighter than `=` in a phrase), file-global-words 258e6cda68, raylib-import-failure cdba22378f (`import raylib` loads
   the native lines), bundled-module 2064325354 (a module slice goes on while a text/bracket is open; slices parse with
   their module's uses' operators; the lexer reads a quote with combining marks as a quote), paint-shader 3a545a81c4
   (paint(shader) alone, 5 gpu samples paint), cbrt-refused-pure b0d8489e3d (∛x, cbrt(x)). Sweep before the last
   four: 98/224. Cards found: retire-header-linker, sandbox-hello-trap, macos-build-failure (warp-host on macOS),
   paint-shader-dimensions (the program's own width/height).
   Next groups of the sweep (scratchpad one.sh, 4 at a time): `9 sqrt` / `27 cbrt` postfix and `x.sqrt` method forms
   trap or are refused (test_postfix_words, test_number_methods); `height * width int` (circle samples, a value after
   an assignment); byte_slice from emitter (json_parser, calculator); `[1000: "M", …]` int keys (`:` on int and char,
   arabic-to-roman); `def f(x) = x => …` (y_combinator); `to render text as pdf:` (phrases, return_polymorphism);
   law (laws samples); get routes (servers); match (matching, parked?); python needs libpython.
Parked: `on` handlers (user discussing), stored (handler shape), foreign spellings (user audit).

Sweep script: the session scratchpad sweep.sh (each sample with warp pure, 30 s, one at a time); logs in
~/Library/Logs/warp/samples/sweep.txt.
