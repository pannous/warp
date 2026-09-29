# Float → Int implicit truncation survey (A5)

`emit_truncated_float` (src/wasm_emitter/mod.rs) turns an f64 on the stack into an exact Int with `i64.trunc_f64_s`, silently.
Experiment: scratch worktree `probes/float_trunc_wt` at 637a1677, `emit_truncated_float` replaced by `emit_type_error(...)`
(surfaces as `Error('float value used where an exact Int is expected …')` at eval). No code changed in main.

## Results
- Refuse at all 8 sites: 641 pass / 1 fail of 642 (baseline: 0 fail). Same result when the two explicit-cast sites keep truncating.
- The only existing test that fails: `tests/test_float_parameters.rs::test_float_local_inside_float_function`
  (`half(x:float) := {y=x/2; y+1}`). Reason: site 2 below — the value of the assignment `y=x/2` is pushed in Int context
  (LocalTee, then truncate) although the statement value is discarded. The truncation there is not a semantic read at all, it is a
  wrong-context emit; the fix is to leave the assignment value as f64 (emit in float context), not to keep truncating.
- Nothing else in the suite reaches these sites with a non-integral float; the other sites are effectively only hit by bugs.

## Sites (line numbers of HEAD 637a1677)
| # | line | what | kind |
|---|------|------|------|
| 1 | 1663 | reassign existing float global: value left on stack as Int | implicit (result of assignment expr) |
| 2 | 1681 | first declare of float global: same | implicit (result of assignment expr) |
| 3 | 1900 | `as float` in emit_numeric_cast (float value, truncated to raw Int) | explicit cast, but semantically dubious: `as float` should not truncate |
| 4 | 1950 | `x as int` / `int(x)` of a runtime float expr | explicit cast — truncation is correct |
| 5 | 2205 | `y = <expr>` on an existing float local: LocalTee then truncate (only failing existing test) | implicit (assignment result) |
| 6 | 2293 | `√x` in Int context: sqrt then truncate | implicit (result of sqrt: √2 → 1) |
| 7 | 2358 | float local read in an Int-context operand (`x % 2`, `x == 2`, `x > 1`, `x ^ 2`, `[..][x]`) | implicit read |
| 8 | 2364 | float global read in Int-context operand | implicit read |

Explicit casts: sites 3 (questionable) and 4. Everything else is implicit.

## Silent wrong results today (all fixed loudly by refusing; found with probes in the worktree)
With `f(x:float) := …; f(…)`:
- `x % 2` with 2.7 → 0 (expected 0.7)
- `x == 2` with 2.7 → true (expected false)
- `x ^ 2` with 1.5 → 1 (expected 2.25)
- `x > 1 ? 1 : 0` with 1.5 → 0 (expected 1)  — comparison truncates the float operand
- `x << 1` already refuses today ("cannot extract a numeric value").
Passing today and staying so: `x+1`, `x*x`, `-x`, `abs x`, `int(x)`, `x as int`, `x as float`, `{y=x; y=y+1; y}` (this last one refused: site 5).
Note: top-level `x=2.7` is an exact Quotient, not a float local, so it never reaches these sites; only `x:float` params and
locals/globals of float kind do. `[..][2.7]` and `"abc"[2.7]` give `index out of range` (separate path, not this function).

## Proposed rule
1. An f64 never converts to Int implicitly. Only an explicit `as int` / `int(x)` (site 4) truncates; give it its own
   function (`emit_cast_to_int`) and delete `emit_truncated_float`.
2. Assignment expressions (sites 1, 2, 5) yield their value in the variable's own kind; the consumer converts. Unused results are dropped.
   This alone fixes the one failing existing test.
3. Int-context reads of a float (sites 6, 7, 8): if either operand of an arithmetic/comparison op is float, the op is emitted as float
   (mixed promotion, `f64.rem`/pow/`f64.lt` …) — this makes the four silent wrong results above correct instead of merely loud.
   Where no float operation exists (bit ops, `<<`, index with non-integral value) → compile error (type_errors), not truncation.
4. Site 3 (`as float`): should be an identity/promotion to f64, not a truncation; check what its Int result is used for before changing.
Order: do 2 first (unblocks test), then 3, then flip `emit_truncated_float` to an error as the safety net.

Worktree `probes/float_trunc_wt` is untracked scratch (it can be removed with `git worktree remove --force probes/float_trunc_wt`).

## Outcome (A5 implemented, all sites closed)

Final rule: **a float never becomes an exact Int implicitly.** Only the explicit `x as int` / `int(x)` truncates
(`emit_truncating_cast`, which also serves floor/ceil/round through `emit_integral_float_as_int`); a NaN or a magnitude ≥ 2^63
fails with the runtime error `float out of int range` because there is no f64 → bignum path. Everything else that used to truncate
is now `emit_float_in_exact_context`: "<x> is a float where an exact Int is expected: use it in float arithmetic or truncate with `as int`".
Mixed float/Int arithmetic and comparisons promote to f64; `%` is euclidean (`^`: libm `m.pow`, fractional literal exponents make the
power a float, NaN traps as `invalid number`).

| # | site | now | commit |
|---|------|-----|--------|
| 1, 2 | float global assign/declare | float-in-exact-context error | bab7a22b |
| 3 | `as float` in an exact context | error, `as float` promotes in float context | a8f17bdf |
| 4 | `x as int` / `int(x)` | the only truncation, range-checked | bab7a22b, ccaf3da9 |
| 5 | assignment to a float local | keeps its f64; dropped statements via `emit_discarded_statement` | 40ac69a3 |
| 6 | `√x` in an exact context | error | bab7a22b |
| 7, 8 | float local/global read as an Int operand (index, `and`, `&`, …) | error | bab7a22b |
| – | `x % 2`, `x == 2`, `x ^ 2`, `x > 1` with a float operand | computed as f64 | 506e6b94, fd4557dc |
| – | floor/ceil/round (5 copies of trunc + box) | one helper, range-checked | ccaf3da9 |
| – | FFI f64/f32 result read as an Int | error | ecc7847c |
| – | floor/ceil/round/count in a function body | share the integer-builtin path with top level | 9be02d9b |

Open: shifts do not exist (`<<` parses as `<` plus an angle group, see TODO.md); there is no exact Int for floats beyond i64.
The scratch worktrees `probes/float_trunc_wt` and `probes/stage_check` are obsolete.
