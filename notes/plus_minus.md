# ± values: numbers with uncertainty (card plus-minus)

User (card): "allow `x ≈ radius±1`, hopefully similar to Julia, which just passes through all the functions without
needing any modifications to the system at all". Julia's Measurements.jl: `x = 5 ± 1` is one number with an
uncertainty; every generic function computes with it, the uncertainty propagates (linear error propagation).

## Today (2026-10-08, probed)
- `±` (also ` +- `) parses as Op::PlusMinus. units.rs evaluates `1950 ± 50` and `1950 ± 50 cm` at compile time as a
  Tolerance (integers, printed as written); `1900 - 2000 AD == 1950 AD ± 50` waits for a user decision
  (notes/ignored_tests.md).
- Any arithmetic on it is the error "arithmetic on a value with tolerance or a range is not supported", `r ± 0.1` of a
  variable or a float is a parse error, `sqrt(4 ± 1)` too, `12 ≈ r±1` the same error.
- `≈` (notes/approximately.md): numbers within 1e-9 relative (`tolerance`), field by field for objects.

## Meaning (defaults, undoable; questions queued with the Interviewer)
1. Propagation as Measurements.jl, first-order (linear) Gaussian: f(x ± σ) = f(x) ± |f'(x)|·σ; independent
   uncertainties add in quadrature: `(5 ± 1) + (2 ± 1)` is `7 ± 1.4`. Alternative: interval arithmetic
   (worst case, `7 ± 2`, sound bounds, grows fast).
2. Correlation is tracked, as in Measurements.jl: each `±` written is one independent source; a value keeps its
   contribution per source (∂f/∂source · σ_source), so `x - x` is `0 ± 0`, `x * x` is `2x·σ`, not `√2·x·σ`.
   σ = √Σ contribution².
3. `a ≈ b ± e` holds when |a - b| ≤ e; with both sides uncertain, when |a - b| ≤ √(σa² + σb²) (1σ, the tolerance as
   written). `≈` of plain numbers stays as D8. `a == b` compares value and σ exactly (`x - x == 0 ± 0`); `<` compares
   values.
4. Printing: σ to 2 significant digits, the value to the same decimal place: `7.0 ± 1.4`, `3 ± 1`, `1950 ± 50`
   (integers stay integers when value and σ are integers). Alternative: all digits like Julia (`7.0 ± 1.4142135623730951`).
5. `x.value` (alias `x.nominal`), `x.uncertainty` (alias `x.error`, `x.σ`): plain numbers. A plain number is `n ± 0`
   where mixed.
6. The compile-time Tolerance of units.rs (`1950 ± 50 AD`) is the same value: one meaning of ±. The range question
   (`1900 - 2000 AD == 1950 AD ± 50`) stays open as it is.
7. Never silent: a function that cannot propagate (an FFI call, `as int`, a parameter typed `x: float`) is a loud
   error naming the ± value ("the C function f takes a float, got 3 ± 1; fix: f(x.value)"), never a dropped σ.

## Where it runs (implementation plan)
Static kind, run-time struct (like static units, notes/units_runtime.md, but the uncertainty itself is run-time data):
- Analyzer: `Kind::Uncertain`, inferred like Float: a `±` literal has it, arithmetic with a number keeps it,
  user functions are specialised per argument kind where an argument is uncertain (as static_units specialises per unit
  signature), so `f(x) := x*x + 1; f(3 ± 0.1)` needs no change to f ("passes through all functions").
- Run time: GC struct `$Uncertain { value: f64, sources: (ref $Contributions) }`, contributions an array of
  (source id i64, contribution f64) sorted by id; ids from a global counter at each `±` evaluated. Runtime functions
  `uncertain_add/sub/mul/div/neg`, `uncertain_scale(x, value, derivative)` for unary functions, `uncertain_sigma`,
  `uncertain_text`. A Node of kind Uncertain (data = the struct) for lists, maps, any.
- Math words with derivative rules: sqrt, exp, log, sin, cos, tan, pow/`^` with a constant or uncertain exponent, abs.
- Reading back (wasm_reader): a new Node form or `Node::Key(value, PlusMinus, σ)` as today's Tolerance prints.
- Steps (each a small branch): (1) literals of numbers and variables, + - * / neg, printing, `.value`/`.uncertainty`,
  `≈`; (2) correlation (sources) if not already in 1; (3) math words; (4) user functions specialised; (5) lists/any as
  Nodes; (6) units together (`5 m ± 1 cm`).
- Effort: L (about the size of static units stage 1+2).

## Done (branch plus-minus, 2026-10-08)
Built differently from the plan above, simpler: no static kind and no specialising. `a ± b` is a run-time value,
Kind::Uncertain = 17, analyzed as Kind::Data, so every `+ - * /` that meets one goes through node_add/sub/mul/div
(list_ops.rs), and untyped functions pass it through unchanged (`f(x) := x*x; f(3 ± 0.1)` is 9.00 ± 0.60).
- Run-time form (wasm_emitter/uncertain.rs): a $Node of Kind::Uncertain whose data is one f64 array
  [value, source id, contribution, …], sources ordered by id, ids from a global counter at each `±` evaluated. No new GC
  type. uncertain_combine merges two source lists scaled by the partial derivatives, so `x - x` is 0 ± 0.
- `≈`: values_similar hands an uncertain side to uncertain_similar: |a - b| ≤ σ(a - b), correlation included, or
  numbers_similar.
- Reading back: Node::data(uncertain::Uncertain {value, sigma}), shown by its Display: σ to 2 significant digits.
- units.rs declines (Stop::Unsupported) `+ - * /` on a unit-less tolerance, so those programs compile normally; the
  span comparisons (`1950 ± 50 == 1900 - 2000`) and every case with units stay compile-time as before.
- Tests: tests/numbers/test_plus_minus.rs.
Not yet (step 1 rest): printing at run time (`print x`, `"{x}"`: text_of has no Uncertain case), `.value` and
`.uncertainty`, negation `-(5 ± 1)` (units.rs still errors), comparisons `<`, the playground reader (web.rs), math
words (step 3), units (step 6). A typed parameter `f(x: float)` refuses an uncertain argument (not a number).
