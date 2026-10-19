# ratio-gcd-warp: the exact integer runtime in warp

Card ratio-gcd-warp: src/wasm_emitter/numbers/exact.rs int_gcd, ratio_new, exact_div, int_mod and int_euclid_div
(~165 lines of emitted wasm) as warp.

## Step 1 (sprint 2026-10-19; tested the same day)
lib/math.warp holds the twins, typed: `gcd` (now Euclid's loop, never below 0, like int_gcd; before, `gcd(-4, 0)` was
-4), class `RatioParts { numerator: int, denominator: int }`, `lowest_terms(n, d)` (ratio_new's normalization:
positive denominator, 5/0 is 1/0, 0/0 stays), `exact_quotient(a, b)` (exact_div, by 0 "divide by zero"),
`euclidean_remainder` and `euclidean_quotient` (int_mod, int_euclid_div from `rem`). The Rust builtins are unchanged.
Test: tests/numbers/test_ratio_gcd_warp.rs compares them with `/`, `%`, `/!` (green). gcd's absolute values read
`0 - a`: `if a < 0 then -a else a` inside a block fails "undefined variable: else" (card inside-block).
`gcd(-4, 0)` is 4 as in mathematics (gcd ≥ 0) and Rust's int_gcd; no test or page pinned -4. Benchmark: probes/ratio_gcd/bench_builtin.warp
vs bench_warp.warp (`warp compile --wasm` each, `hyperfine -N`).

## Why the builtins stay (for now)
- Every program with int `/` links ratio_new and int_gcd; a late call into a prelude word costs ~10 ms of compile per
  program (notes/self_hosting.md "Load order") until lowered prelude words are cached (card cache-lowered).
- Circular: the twins use `/!` and `rem`, which compile to int_euclid_div / int_rem. A late call from int_euclid_div into
  euclidean_quotient recurses; the wiring must keep the truncated int_quot/int_rem (the fixnum fast paths) in Rust and
  move only the slow path, or give warp a truncated-quotient word.
- ratio_new's $Ratio layout (inline small ratios, int_store) is CORE: warp computes the parts, Rust builds the value.

## Next step
1. Measure bench_builtin vs bench_warp (not run yet).
2. Then, if cache-lowered landed: int_gcd's slow path (an operand beyond the fixnums) calls prelude gcd late, the i64
   loop stays inline; the same for int_mod/int_euclid_div slow paths. The words move from math.warp to prelude.warp
   (late calls load prelude words only).
