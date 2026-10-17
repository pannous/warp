# Symbolic reals at run time: design sketch (2026-10-14, card follow-up of decision real-runtime)

Decision real-runtime (2026-10-13): real.rs (compile-time symbolic reals, 830 lines) stays until symbolic reals are a
run-time number type next to the exact ratios; then it goes. This is a sketch only: nothing is built yet.

## Goal
`f(x) := x*x; f(√2)` is 2, not 2.0000000000000004. Today real.rs answers only constant programs; any function, loop or
input lowers π, √2 … to f64. With a run-time type, every program computes in the same normal form as
extensions/reals.rs `Exact`: a sparse polynomial with rational coefficients over the generators π, ℯ, ⅈ, √r and ∛r
(ε later, decision P57).

## Representation (WASM GC)
It reuses the exact-number heap. An Int handle can already point into `$Numbers` at a `$BigInt` or a `$Ratio`, and
it may now also point at a `$Real`. Integers and ratios keep their fast paths. The slow paths ask `is_real` next to
`is_ratio`, the same branch exact_dispatch already takes.

```
$Factor = (struct (field $generator i32)    ;; 0 π, 1 ℯ, 2 ⅈ, 3 √, 4 ∛ (5 ε later)
                  (field $radicand anyref)  ;; Int payload for √/∛ (square/cube-free, > 1), null otherwise
                  (field $exponent i64))
$Monomial = (array (ref $Factor))           ;; sorted by (generator, radicand), no zero exponents: the print order
$Term = (struct (field $coefficient anyref) (field $monomial (ref $Monomial)))  ;; coefficient: Int payload or $Ratio
$Real = (array (ref $Term))                 ;; sorted by monomial, no zero coefficients; never a bare rational
```
Invariant, as in `Exact`: a value whose only term has the empty monomial is returned as its Int or $Ratio. That keeps
`√2*√2` an Int, and keeps equality structural.

## Operations (emitted run-time functions, function_builder::runtime_function)
- `real_add` / `real_sub`: a merge of two sorted term arrays, adding the coefficients with exact_add.
- `real_mul`: every term pair. Monomial product: merge the factors and add the exponents. Then normalise:
  - ⅈ² = -1
  - √a·√b = √(ab), with the square factor pulled out
  - ∛ the same way, with cube factors
  This needs int_gcd plus radical reduction by trial division up to 10^4, the same limit reals.rs uses.
- `real_div`: only by a single term (monomial inverse: negative exponents for π/ℯ, √r⁻¹ = √r/r). A sum in the
  denominator, `1/(1+√2)`, falls back to f64 with a warning, as reals.rs approximates today.
- `real_compare`: interval arithmetic in fixed point, as reals.rs does: Machin for π, a series for ℯ, isqrt/icbrt.
  It runs at 64 bits, then 4096. Still undecided, it raises the trap/error `undecidable`.
  This is the largest part (~300 lines of emitted wasm). Alternative: compare via f64 when the gap is clear (one
  f64 subtraction), and do the fixed-point refinement only when the gap is tiny.
- `real_equals`: structural, on the normal form (the arrays are compared item by item).
- sin/cos/tan/exp/ln: exact only at the table points (multiples of π/12, ℯ^n). Each is a lookup on the term shape,
  else f64, which turns the value into a float as any float source does (exact.rs rule "mixing one in makes f64").

## Printing
`real_text`, beside exact_text, prints the same form as `Exact`'s Display (`π√2`, `(√6-√2)/4`, `1+ⅈ`), so a
compile-time answer and a run-time answer print the same. reals.rs Display is the oracle: a differential test
evaluates the same expressions through both paths and compares the texts (the Lean HyperGeneral model stays an
optional second oracle).

## f64 fallback
- `x as float`, a float operand, or an unsupported function gives f64. The conversion sums the terms with each generator
  approximated (the generator table is a constant f64 array).
- A declared `float x = π` converts (decision #11).
- Hyperreals (ε) are refused before conversion, as `Generator::approximate` refuses them now.

## Reading back (gc_traits / wasm_reader)
An Int node whose payload is a `$Real` reads back as `Number::Real(Real::Exact(..))`, as a `$Ratio` reads back as
`Number::Quotient`. Then is!/eval need no change for tests.

## Migration and size
1. Emit `$Real` and add/sub/mul/equals/text. Literals π, ℯ and √n become constants of the real heap, not f64.
2. real_compare; then the trig/exp table.
3. real.rs answers fewer and fewer programs. Once the run-time path passes its tests
   (tests/numbers real*, samples using π/√), delete real.rs (−830), keeping extensions/reals.rs as the compile-time
   constant folder and the oracle.

Estimate: +900 to +1200 emitted-runtime lines (exact.rs is 1444 for ratios and big ints), −830 for real.rs, so
roughly net +200 to +400 for the feature "exact reals in every program".
Risks:
- Code size of programs that touch π: the runtime is emitted only when used, as big_int is today.
- Compare cost: needs the fixed-point series in wasm.
- Browser parity: everything is emitted wasm, so no host imports are needed.

## Parked (user 2026-10-14: "park a generalized exact mechanism for later")

Not built for now: real.rs stays as it is, card cleanup-real-answer is in Later. The open questions below are parked
with it (3 was for later anyway).

## Open questions (parked)
1. Is a sum in a denominator, `1/(1+√2)`, an f64 with a warning, or should it be rationalized (conjugates) at run time?
2. An undecidable comparison at run time: a trap with a message, or an Error value?
3. Should ⅈ be allowed at run time (complex reals) in step 1, or only real generators first?
