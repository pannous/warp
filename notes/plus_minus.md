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

## Meaning (user decisions P217–P220, notes/decisions.md)
- P217 an interval, worst-case bounds, not Measurements.jl's Gaussian: `(5 ± 1) + (2 ± 1)` is `7 ± 2`, `x - x` is
  `0 ± 2` (no correlation), functions map the endpoints: `sqrt(4 ± 1)` is √3..√5, shown `2.00 ± 0.27`. Gaussian later
  through an explicit form such as `5 ± 1σ` (card plus-minus-gaussian).
- P218 `x ≈ r ± 1` holds when |x - r| ≤ 1; two ± values are ≈ when their intervals overlap.
- P219 the ± part to 2 significant digits, the value to the same place: `7.0 ± 2.0`, `3.00 ± 0.50`.
- P220 one meaning of ±: `1950 ± 50 AD` (units.rs Tolerance, compile time) is the same interval value.
- Default (undoable): never silent: a function that cannot take an interval (FFI, `as int`, a parameter typed
  `x: float`) is a loud error naming the ± value, never a dropped bound.

## Where it runs (branch plus-minus, done)
No static kind and no specialising: `a ± b` is a run-time value, Kind::Uncertain = 17, analyzed as Kind::Data, so every
`+ - * /` that meets one goes through node_add/sub/mul/div (list_ops.rs), and untyped functions pass it through
unchanged (`f(x) := x*x; f(3 ± 0.1)` is 9.00 ± 0.61).
- Run-time form (wasm_emitter/uncertain.rs): a $Node of Kind::Uncertain whose data is the f64 array
  [value, low, high]; no new GC type. uncertain_add/sub/mul/div: the value from the values, the bounds the least and
  greatest of the operation on the four endpoint pairs (division by an interval holding 0 gives infinite bounds).
- `≈`: values_similar hands an uncertain side to uncertain_similar: the intervals overlap, or numbers_similar.
- Reading back: Node::data(uncertain::Uncertain {value, low, high}), its Display shows value ± the farther bound.
- units.rs declines (Stop::Unsupported) `+ - * /` on a unit-less tolerance, so those programs compile normally; the
  span comparisons (`1950 ± 50 == 1900 - 2000`) and every case with units stay compile-time as before.
- Tests: tests/numbers/test_plus_minus.rs.

- Card plus-minus-print (done): print and `str(x)` show `6.0 ± 1.0` (uncertain_text in list_join, the format of
  the read-back); `x.value`, `x.low`, `x.high`, `x.uncertainty` as floats (uncertain_field, in the constant-field
  lookup like `e.message`); `-x` of any run-time number is node_sub(0, x); `<` and the other orders compare the values
  (node_order), an undoable default queued with the Interviewer (alternative: `<` only when the whole interval is below).
  Tests: tests/numbers/test_plus_minus_print.rs.

## Open
- `"y=" + x` with an interval x is the general type error text + data (no implicit conversion of a run-time value);
  `"y=" + str(x)` works. Text interpolation does not exist in warp (notes/i18n.md).
- Card plus-minus-playground: the playground reader (web.rs node_from_tree), math words (sqrt, sin: map the endpoints,
  split at extrema), units with ± at run time (`5 m ± 1 cm`).
- A typed parameter `f(x: float)` refuses an interval argument ("not a number"); the error should name the ± value.
