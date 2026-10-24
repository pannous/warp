# backup/pure-pipes (parked 2026-10-24, Worker 2, cards hDLe80 / hDLRbM)

State: one wip commit (f0af29847) on top of pure-pipes 0c9660041 (already on main), adding
tests/operators/test_pipe_functions.warp; origin/main merged in cleanly. Warp-only, no Rust.

Why parked: the test file passes in no lane yet, so it is not committed to main (supervisor handover: "commit it if
it passes").
- `WARP_SELF_HOSTED=pure warp test tests/operators/test_pipe_functions.warp` (the lane test_pipe_shadowed.warp passes
  in): 1 of 6 passes. The Rust front end reads `inc | dbl` and `3 | inc` as the logical or ("inc needs 1 argument");
  the sugar rules of lib/sugar/calls.warp don't apply there. Rust is frozen, so this lane waits for the pure pipeline.
- `warp pure` (compiler.wasm, after `warp rebuild pure`) of the same cases as a program: 5 of 6 give the expected
  value (8, 5, 4, [4 6], yes). The miss: `" ab " | trim` is "outside the core: the unknown name trim": the rule
  `$x | $f => $f($x) where $f takes one` needs a library word whose first line takes one typed value, and
  lib/prelude.warp declares `trim(t): text from emitter` untyped.

Next steps:
1. Type the prelude's text builtins (`trim(t: text): text from emitter`, beautiful-types), check `warp pure` gives
   `ab` for `print(" ab " | trim)` and that the self check stays green.
2. Land the test file once a lane runs `.warp` is! files through compiler.wasm (or the pure lane applies the sugar).
