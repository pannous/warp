# Number literals: decimals are exact

- A decimal literal is exact (decision exact-decimals: inline i64 mantissa + power-of-ten scale), so `0.1 + 0.2 == 0.3`
  is yes. f64 only comes from float sources: `1.5f`, `as float`, √, π-free irrational results and FFI.
- A whole decimal is the integer (P196b): `1 === 1.0` is yes. Type names: `int` (also `2.0`), `rational` (exact
  fractions and decimals), `real` (π), `float` (approximations).
- A float never becomes an exact Int implicitly. Only `x as int` / `int(x)` truncates (`emit_truncating_cast`,
  range-checked: `float out of int range`); any other float in an exact context is the error "… a float where an exact
  Int is expected"; mixed float/Int arithmetic and comparisons compute in f64. `int i=3.3*2` is the declared-type
  mismatch, as `int i=3.5`.

```warp
type(0.0)          # int
type(1.5)          # rational
type(3/2)          # rational
type([0.0, 0.0])   # list of int
0.1 + 0.2 == 0.3   # yes; `==` warns only when a float is involved
```

## A later float write widens the type everywhere

A variable's type is the join of its literal and every write the analyzer can see, so a read anywhere sees the final
type: `shown = [0.0, 0.0]; shown[0] = sqrt(2.0)` makes `shown` a list of float from the start, and an int variable
later given a float holds floats throughout. `collect_variables` collects again with the widened types until nothing
widens (`Scope::widened_bindings`). Without it, a read typed before the widening write (`max` lowers to
`(a = …; b = shown#i; …)`) got an i64 local fed an f64: "WASM validation failed: expected i64, found f64".
Not covered: writes the analyzer cannot see (a list passed to a function that writes floats into it), which would need
widening at run time, a check on every store.

## Repeating decimals

`0.3̅` (U+0305 COMBINING OVERLINE after each period digit) is the exact ratio 1/3: parser
`repeating_decimal_literal` (src/warp_parser/literals/mod.rs) → `Number::repeating_decimal` (n = integer·fixed·period −
integer·fixed, d = 10^|fixed| · (10^|period| − 1)). No parentheses form `0.(3)`:
`0.5(3)` stays juxtaposition, 1.5. A plain digit after an overlined one (`0.3̅4`) is an error ("Unexpected character").
`x as decimal` is a Text (wasm runtime `decimal_text`, src/wasm_emitter/numbers/exact.rs): the period found as the order of 10
modulo the denominator without its 2s and 5s, long division writes the digits; a terminating ratio prints plainly
(`0.25`), a period of 1000 digits or more prints as the ratio (`1/7919`), a float as its shortest text.
