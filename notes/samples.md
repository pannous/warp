# Sample sweep (samples/, started 2026-10-03)

Goal: every `samples/*.wasp` runs, ideally by fixing the language rather than the sample. Each fixed sample gets an
`is!("samples/x.wasp", …)` line in tests/programs/test_samples.rs.

## How to sweep
- Build a private CLI copy: `WARP=$(scripts/own-warp.sh)` then run `"$WARP" …` (or `probes/samples/…`).
  The shared `debug/warp` is not hashed per checkout: another worktree's build overwrites it between your build and your run.
- Skip `raylib_*` / `sdl_*` in sweeps: they open real windows.
- `test.wasm` in the cwd is the last emitted module, written before validation: `wasm-tools print test.wasm` shows the
  function behind an "internal error: WASM validation failed".
- `samples/life_kotlin_ranges.wasp` fails on purpose (tests/welcoming/test_welcoming_ask.rs pins its explanation).

## Wasp habits the old samples get wrong (sample-side fixes)
- Lists are values: a function cannot change a list it is passed. Return the new list, or make the state `global`.
- A function sees main-level constants; to change (or read a computed) main-level value, declare it `global`.
- `/` is exact division (`7/3` stays a fraction); use `//` for floor division, written glued: ` // ` with spaces is a comment.
- `1e10` is an exact number and cannot later hold a float: `1e10 as float`.
- One-character texts are characters: `'5' as int` is 53; `c as text as int` is the digit.
- Types are declared `type V { x: float }` (not `type V: {…}` / `type V = {…}`); construct with `V(1, 2)`, `V{x: 1}` or `V { x: 1 }`.
- No keyboard/mouse input, no `pop`, no `Array2D`: interactive games become self-playing demos.

## Language fixes made for the samples (see git log of branch `samples`)
replace word, `x.chars()` list, suffix words vs variables (`solved`), one run state for host+WASI+FFI imports, host
words sleep/random/clock, run-time float text, comma tuples of computed items, list statements in loop bodies, globals in
return-kind inference, `and`/`or` decided at run time for calls and Nodes, `if` of two characters, spaced construction,
run-time field reads of declared fields, `{x: x}` keys, call shapes, declared field kinds, float parameter widening,
float-path conditionals/returns/zero-arg calls, globals and captures typed with function kinds.

Later batches: records (optional fields, field_with on instances, bare return, recursive list functions), nested block
comments, C-style `real f(real x) {…}` definitions and `if (c) statement`.
- Output filters: `grep -v '^      '` (to hide hint continuation lines) also hides indented program output.
- `tau`/`τ`, `pi` are constants: a variable cannot be named so.
- `real` is the exact real type; use `float` for IEEE arithmetic.

Branch samples-2 / implicit-libm (warp-d2): a character variable compares by code point (`c >= '0'`), words
`is_digit` `is_alpha` `is_alphanumeric`, method syntax for builtins (`x.round()`, `x.floor()`, `x.sin()`; user
functions since 3da746b), `x="5"; x as int` is 5, and libm called without import links libm (exp/sin/… compiled to
their argument at run time before).

## Still failing (2026-10-08, branch samples-run)
All other samples run (exit code = the program's value, so sweep for `Error(` in the output, not for exit codes).
The failures left, each with its reason in web/playground/excluded_samples.txt:
- errors (malformed on purpose), life_kotlin_ranges (fails on purpose), modules (sketch: card module-manager),
  webgpu (WGSL block), netbase (needs a netbase server).
- natural: given its data it runs to `send email to … with subject …`; that phrase and `results where country is
  germany` wait for decisions (card natural-phrases), `10 down to 1` is card down-wasm.
- raylib/sdl: not run in sweeps (windows). SDL_video.h is among the SDL headers now (SDL_CreateWindow).
Fixed on samples-run: a loop variable reusing a number's name reads fields (`item = 2; for item in basket {
item.price }`); an import no header declares says so; `xs where country is germany` hints `it.country==germany`.
