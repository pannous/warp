# Hyperreal numbers ε, ω (wiki/hyperreals.md, wiki_features row 36)

Representation: the exact-real normal form (src/extensions/reals.rs) gets one more generator, `Generator::Epsilon`,
with any integer exponent; ω is ε⁻¹ and prints as ω, ω². A hyperreal is a Laurent polynomial in ε whose coefficients
are exact reals (ℚ with π, ℯ, √, ∛), the same shape as the Lean model (`R*`: ℚ-coefficient list of ε powers,
~/dev/script/lean4/hyper). It is the smallest sound representation: + - * and integer powers are closed and exact.

- Parser: the glyphs `ε` and `ω` only (`epsilon`/`omega` stay free names, like `e` and `i`).
- Order (`Exact::sign`): the terms with the lowest ε power decide (ω terms outweigh reals, reals outweigh ε terms);
  their real coefficients are signed by the usual interval arithmetic. So `ε>0`, `ε<0.0001`, `ω-1000<ω`, `ε²<ε`.
- `st(x)`: the standard part (the ε⁰ terms); `st` of an infinite number (an ω term) is an error.
- Inverse: exact only for a single term (`1/ε = ω`, `1/(2ε) = ω/2`). `1/(1+ε) = 1-ε+ε²-…` needs infinitely many terms:
  an error, never a truncation (the Lean side's truncated geometric series, HyperApproxInv.lean, is not adopted yet).
- No silent approximation: a hyperreal has no f64. `ε as float`, `sin(ε)`, `√ε`, a comparison with an approximation
  and ε in non-constant programs (functions, loops: the WASM path lowers reals to f64) are loud errors.
- Evaluated by `real::answer` at compile time like every exact real; there is no WASM GC representation yet.

Open (asked the Interviewer): inverse of sums by truncated series, dual-number mode (ε² = 0), `≈`/halo (D8),
hyperreals at run time.
Tests: tests/numbers/test_hyperreals.rs.
