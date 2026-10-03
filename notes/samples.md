# Sample sweep (samples/, started 2026-10-03)

Goal: every `samples/*.wasp` runs, ideally by fixing the language rather than the sample. Each fixed sample gets an
`is!("samples/x.wasp", …)` line in tests/test_samples.rs.

## How to sweep
- Build, then copy the binary at once: `cargo build --offline --bin warp && cp ~/.cargo/shared-target.noindex/debug/warp probes/samples/warp`.
  The CLI binary is not hashed per checkout: another worktree's build overwrites it between your build and your run.
- Skip `raylib_*` / `sdl_*` in sweeps: they open real windows.
- `test.wasm` in the cwd is the last emitted module, written before validation: `wasm-tools print test.wasm` shows the
  function behind an "internal error: WASM validation failed".
- `samples/life_kotlin_ranges.wasp` fails on purpose (tests/test_welcoming_ask.rs pins its explanation).

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

## Still failing (2026-10-03 night)
async, calculator, circle, control_flow, data_structures, errors, functions, html, html_dsl, json_parser, mandelbrot,
modules, natural, netbase, neural_net, particles, polymorphism (needs parameter overloading, todo.md), sample,
test_ffi_extended, types, wasm_interop, webgpu; raylib/sdl not run (windows).
