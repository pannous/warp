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
  lookup like `e.message`); `-x` of any run-time number is node_sub(0, x). Sorting orders intervals by their values
  (node_order). Tests: tests/numbers/test_plus_minus_print.rs, sample samples/measurements.warp.
- Comparisons (P224, P224b): `y certainly < x` (the whole interval: y.high < x.low), `y possibly < x` (some of it:
  y.low < x.high), also `certainly(y < x)`; crate::uncertain::lower_certainty makes the infix form the builtin call,
  wasm_emitter/uncertain.rs uncertain_order answers it. A bare `<` `>` `<=` `>=` on a value the compiler knows to be ±
  (a variable given `a ± b` or arithmetic with one) is a compile error offering the three readings
  (uncertain::check_orderings). Where it can't know (a parameter, data read at run time) the bare ordering is
  `certainly`: an overlap counts as no and warns once per run through host.warn, naming the line; a program that
  makes ± values and orders values therefore imports the host (may_order_intervals).

- Card plus-minus-playground (done): math words map an interval (P217). √ ∛ abs and the libm functions of one
  argument (crate::uncertain::INTERVAL_WORDS), of a value held as a Node in a program with ± values, call
  uncertain_<word> (wasm_emitter/uncertain.rs): f of a number as before, of an interval f at both ends, least and
  greatest, and an extremum or pole the interval reaches is the bound (sin's ±1 at π/2 + 2πk, tan's ±∞ at its poles,
  cosh's and abs's least at 0). The analyzer types the word's result a Node when its argument is one. `(y)` of a
  variable now has the variable's kind (it read as the call y()). The playground reader (web.rs) reads ± values back.
  Tests: tests/numbers/test_plus_minus_math.rs.

## ± with units at run time (card plus-minus-units, 2026-10-09)
`x = 5 m ± 1 cm; x * 2` gives `10.000 ± 0.020m`. units.rs lower_run_time_tolerances turns a written `v ± s` with a unit
into `quantity(amount ± spread, "unit")` (spread converted to v's unit, a plain spread counts in it, another dimension
is a DimensionError), but only when units::answer can't answer the whole program (RUN_TIME_TOLERANCE_ERRORS or
Unsupported), so span comparisons and final-value tolerances stay compile time. Quantity's `amount:number` admits a ±
value (type_tests.rs runtime_kind_mask; `float` stays strict). Function parameters get their class from the calls
(class_methods.rs parameter_classes), so `f(x) := x * 2; f(5 m ± 1 cm)` dispatches to Quantity.times.
A final Quantity value shows its text (card instance-final). `f` called with a quantity and with a plain number calls
a copy `f_Quantity(x:Quantity)` for the quantity (card mixed-arguments, class_methods.rs specialized_calls). Tests: tests/numbers/test_plus_minus_units.rs, samples/measurements.warp.

## Quantities with tolerance (card quantity-tolerance, 2026-10-09)
- `rope certainly > 4 m`: lower_certainty runs before class_methods, whose with_certain_amounts compares
  `rope.amount` with `same_dimension(rope, 4 m, "compare").amount`, so the interval survives (Quantity.more is a plain yes).
- `rope.low/.high/.value/.uncertainty`: Quantity methods (lib/units.warp) over `bound_of`; the names are
  uncertain::INTERVAL_FIELDS, renamed and dispatched by class like library-word methods, so `x.low` of a ± number stays
  the field. A plain Int/Float reads as the exact interval [x, x] (uncertain_field).
- `sum([5 m ± 1 cm, 3 m ± 2 cm])`: a literal list of run-time quantities folds `reduce((s, i) => s.plus(i))`.
- `Rope(5 m ± 1 cm)` of a unit field: unit_fields::written_quantity reads `quantity(a, "m")` back as `a * m`.
- A Gaussian quantity shows `12.00 ± 0.60σ m`.
- A field type with a tolerance (P233, card field-tolerance): `class Part{length: m ± 1 mm}`, `Part(5 m).length` is
  `5.0000 ± 0.0010m`; unit_fields::lower_field_tolerances (a source pass before lower_run_time_tolerances) writes the
  spread into each construction (`Part(5 m)`, `Part{length: 5 m}`) and leaves the field type `m`; a value given with its
  own tolerance keeps it.
- A final ± value of a unit field (quantity-final) is a units::UncertainQuantity: static_units::quantity_of scales the
  Uncertain to the field's unit, `Rope(5 m ± 1 cm).length` shows `5.000 ± 0.010m`.
- `str(Rope(5 m).length)` is `5m`: casts::computes_value makes str of a field read or of literal arithmetic compute
  the value instead of serializing the code (str-inline).
- `s = sum([quantity("5 m"), …])` is a Quantity (quantity-sum): class_methods::quantities_reduced marks the fold
  TypedAs Quantity and instance_classes gives the variable that class, so `s * 2` is `16m`.
- `lo(a:number) := a.low; lo(5 ± 1)` (number-param): in a program with ± values, uncertain::lower_interval_parameters
  marks a `number` parameter type with IntervalNumber, which annotated_kind reads as a Node parameter (Kind::Empty)
  with the run-time mask of `number`; a wrong argument says "needs a number".

## Gaussian ± (card plus-minus-gaussian, 2026-10-09)
`5 ± 1σ` or `5 ± 1 σ` (the spread times the symbol σ, joined like a unit word: card trailing-symbol) is a Gaussian: one standard deviation; a bare `5 ± 1` stays an
interval (P217). Its parts are [value, low, high, σ, id₁, c₁, id₂, c₂, …]: each contribution c = ∂/∂xᵢ·σᵢ of an
independent source xᵢ (a fresh id per written `±σ`), σ = √Σc² (Measurements.jl's linear propagation with
correlations), so `x - x` is `0 ± 0σ` and `x * x` doubles the relative error. Arithmetic: wasm_emitter/uncertain.rs
gaussian_add…div → gaussian_combine (merges contributions by id); math words use the numeric slope
(f(x+h) − f(x−h))/2h with h = σ·1e-3. low/high are value ∓ σ. Text `7.0 ± 1.4σ`, read back the same.
An interval and a Gaussian don't mix (run-time failure). Tests: tests/numbers/test_plus_minus_gaussian.rs.

## Open
- `if area certainly > 10 then …` parses as `(if area) (then (certainly > 10) …)`: the condition stops at the second
  word; `if (area certainly > 10) then` and `ok = area certainly > 10` work. `x + 1 certainly < 8` reads
  `x + (1 certainly < 8)`; write `(x + 1) certainly < 8`.
- `x + 1 certainly < 8` reads `x + (1 certainly < 8)`; write `(x + 1) certainly < 8`. (`if area certainly > 10 then`
  works since card parser-if-stops: a condition takes a braceless call with a variable argument, as a branch does.)
- `"y=" + x` with an interval x is the general type error text + data (no implicit conversion of a run-time value);
  `"y=" + str(x)` works. Text interpolation does not exist in warp (notes/i18n.md).
- A typed parameter `f(x: float)` refuses an interval argument ("not a number"); the error should name the ± value.
- Gaussian: comparisons (`certainly`, `possibly`) and ≈ treat it as its 1σ interval; units with σ (`5 m ± 1 cm σ`)
  wait: Quantity carries its tolerance as a plain interval (agreed with warp-class).
