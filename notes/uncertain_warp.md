# uncertain-warp: ± values as warp classes (card uncertain-warp)

lib/uncertain.warp (`use uncertain`, registered in src/modules/std_modules.rs) writes the run-time ± of
src/wasm_emitter/numbers/uncertain.rs as typed warp classes with operator methods:
- `Interval { value, low, high }`: plus/minus/times/divide take the worst case of the endpoints (x - x is 0 ± 2),
  less/more are `certainly`, possibly_less/possibly_more the `possibly` forms, approximately (≈) is overlap.
- `Gaussian { value, contributions }` with `Contribution { id, amount }`: linear propagation, contributions of one id
  summed (x + x doubles, x - x cancels), σ = √Σ amount². Ids from `global gaussian_count`.
- Constructors `plus_minus(value, radius)` and `gaussian(value, σ)`; a plain number operand becomes an exact one;
  an Interval with a Gaussian raises "an interval and a gaussian do not mix". Printing goes through the prelude's
  uncertain_text (the same text as the built-in ±).

Written blind in the 2026-10-19 sprint (no build, no test run): tests/modules/test_std_uncertain.rs is unverified.
Not yet moved: the ± literal still builds Kind::Uncertain (the Rust runtime stays); math words through intervals
(INTERVAL_WORDS with extrema, Gaussian slope by SLOPE_STEP); `.low/.high/.value/.uncertainty` fields as plain field
reads (Interval has them as fields, Gaussian as methods); a number on the left (`2 + x`) depends on operator dispatch
finding the right operand's class; division by an interval containing 0 is not special-cased (Rust: check
uncertain_div). Next step: ± literals build these classes (card plus-minus-class), then remove the Rust runtime.
