# Quantities at run time: survey and proposal

## Today (src/units.rs)
`units::answer` evaluates a program of numbers, units and variables at compile time with exact rational amounts
(`1 km/h + 1 m/s` → 23/18 m/s). A program it cannot evaluate takes the normal path, where a unit word is an ordinary
symbol, so it fails as `undefined variable: km` (with the hint "quantities compute only in constant expressions so far").

## Survey (branch runtime-units-survey, tests/numbers/test_units_at_run_time.rs): every case fails loudly, none drops a unit
| written | today |
|---|---|
| `speed(d, t) := d/t; speed(10 km, 2 h)` | undefined variable: km (+ hint) |
| `f(x) := x*2; f(3 m)`, `f() := 5 m; f()` | undefined variable: m |
| `total = 0 m; for i in 1..3 { total += 5 m }` | undefined variable: m |
| `xs = [1 m, 2 m]; xs#1 + xs#2` | undefined variable: m |
| `if c { 5 m } else { 3 m }` | undefined variable: m |
| `print x`, `"d=$x"`, `x as int`, `sqrt(4 m)`, `max(1 m, 2 m)` | undefined variable: m/km |
| `x = 3; y = x m`, `n * 1 km` (variables holding constants) | work (compile-time) |
No silent case was found, so nothing needed a fix beyond the clearer message.

## Proposal: static units (units of measure, F# style), amounts at run time
Units are part of the static type; the run-time value is only the amount, counted in the finest unit of its dimension.
- Type: `Kind::Quantity` plus a unit signature in the analyzer's type (dimension → power, and the display units), inferred
  like any kind: a unit literal has it, `*` `/` combine signatures, `+ - == <` require equal signatures (else the
  existing DimensionError, now at compile time), a number times a quantity keeps it.
- Run-time value: the amount as an exact number (the Int path with ratios, exact.rs), scaled to the finest unit at the
  literal (`2 km` is 2000000 mm); no GC struct and no unit vector at run time, so loops and lists cost nothing extra.
- Functions: a parameter takes the unit signature of its argument; a function called with different signatures is
  specialised per signature (function_values.rs already specialises per function argument); a declared `x:km` checks.
- Output: print/text/return convert the amount back to the written or finest unit (the compile-time signature knows it),
  `as`/`in` conversions are a constant factor at compile time.
- Effort: L. Analyzer kind + signature (~1 day), literal scaling and arithmetic lowering (~0.5 day), specialisation of
  functions by unit signature (~1 day), printing/reading back a quantity (~0.5 day), tests (~0.5 day).

Alternative: a GC struct `{amount: Node, units: (array i64)}` (unit id and power packed per factor) with runtime
functions for * / + compare and print: dynamic like Python's pint, also covers values whose units are only known at run
time (parsed input), but every operation checks and rescales at run time and needs ~8 runtime functions; effort L+.
Recommendation: static units first (sound, zero cost, errors at compile time), the dynamic struct only if input with units
becomes a use case.

## Stage 1 done (branch static-units-1): src/units/static_units.rs
Variables, loops (`total += 5 m`), while loops and if-branches compute quantities at run time: signatures checked at
compile time (DimensionError for `+ - ==` and branches of different units, and a variable given another signature), unit
literals as exact SI amounts, the final value read back in the finest written unit. `compile` of a program whose result is
a quantity is refused for now (only eval reads it back): stage 3 (print/return) covers output.
Next stages: functions (specialised per unit signature), then print/return/as.

## Stage 2 done (branch static-units-2): functions
A call of a user function with quantity arguments (or a body with unit literals) is specialised per unit signature of its
arguments (`speed·u0`), its result signature inferred from the body; a body with unit literals compiles only in its
specialisations. `speed(10 km, 2 h)` → 5 km/h, `twice(3 m)` and `twice(2 s)` are two specialisations. Field names after
`.` are no units (`q.s`). Not yet: recursion with quantities (stays the loud error), names that reuse a unit (`s = …`,
a parameter `h`) leave the program to the compile-time evaluator.

## Stage 3 done (branch static-units-3): output
`print q` and `"text " + q` show the quantity as `str(amount / unit) + " km"` at run time (a fraction with a finite
decimal prints as the decimal: 2.5 km/h), `q as km` / `q in km` check the dimension (DimensionError) and choose the unit
shown, a final conversion decides how the result reads, `return q` carries the signature and all returns of a function
must agree. Still a loud error: lists of quantities, math functions (sqrt, max) of quantities, recursion, string
interpolation (no `$x` interpolation yet), and `compile` of a program whose result is a quantity.

## Stage 4 done (branch static-units-4): lists, sqrt, interpolation
A variable holding a list of quantities has one element signature (`xs = [1 m, 2 m]`; `[1 m, 2 s]` is a DimensionError):
`xs#i`, `sum max min first last` give an element, `count size length` a number, `for x in xs` binds x with the element
signature. `max(a, b)` of quantities too. `√a` halves the powers (`√` of an odd power is a DimensionError). `"${d}"`
shows the unit (interpolation lowers to `text_form(d)`). Still a loud error: a whole list printed or returned, lists
built at run time (`xs.add(1 m)`), recursion with quantities.

## Later: `compile` of a program whose result is a quantity (refused today)
A compiled module returns the SI amount; options to keep the unit:
1. A custom section `warp.units` naming the unit of `main`'s result (`m/s`, with the SI scale), read by wasm_reader and
   any host that wants the quantity; the result stays a number, zero run-time cost, hosts that ignore it see SI amounts.
2. `main` returns a GC struct `{amount, unit text}` (or the text `"5 km/h"`): self-describing for every host, but the
   result type changes for programs whose result is a quantity.
Recommendation: 1, the names section already carries metadata this way ("use WASM names excessively").

## Stage 5 done (branch static-units-5): compiled quantities, whole lists
Option 1 is implemented: a module whose result is a quantity carries the custom section `warp.units` (`km:1 h:-1`),
appended by the pipeline for eval and `compile` alike; wasm_reader (read_bytes, read_bytes_with_imports) and running a
`.wasm` file read it back, so `warp compile …` then `warp out.wasm` prints `500 m`. A whole list of quantities prints and
is a final value as the text `[100 cm 250 cm]` (`join(map(xs, …))` at run time). Still loud: lists grown at run time
(`xs.add(2 m)`), recursion with quantities.

## Revived on main (card units-p64, 2026-10-08)
Stages 1-5 were cherry-picked onto main, about 2600 commits later. The playground's run_module (non-native) now reads
`warp.units` too. A unit word heading a call or command is no unit (`min(n, 3)`, `use m`): before this fix,
lib/list.warp take() was taken for a quantity function and dropped. Still loud errors, not yet metadata: a quantity in a
map or object field (`{dist: d}`), `x:any = d`, `d.serialize()`, lists grown at run time. These need the unit at run time
(the dynamic-struct alternative above): stage 6.

## Stage 6 done (card units-p64): objects, annotations, serialize, growing lists
Static as well, without a run-time struct: a variable holding an object literal with quantity fields notes one signature
per field (`p = {dist: 0 m}`), so `p.dist`, `p.dist += 250 m` and `p.dist = q` check and compute. A final object names
its fields' units in `warp.units` (`dist=m:1;t=s:1`), and reading it back makes those fields quantities. `x:any = q` keeps
the signature. `x:km = q` checks it (P203: annotated code is strict) and x is then a plain number. `q.serialize()` and
`serialize(q)` give the text `"4 m"`. `xs.add(q)` and `xs.push(q)` check the element signature. `dist: 5 m` is data, not a
lazy block (blocks.rs is_computed). Still loud errors: `print p` or a whole object used anywhere but as the final value,
aliases of an object (`q = p`), recursion with quantities. Quantities whose units are known only at run time (parsed
input) would need the dynamic struct.

## Stage 7 done (card units-p64): whole objects and aliases
`print p`, `"${p}"` and `text_form(p)` of an object with quantity fields build its text at run time: the quantity fields
come first, then the plain ones as an object of them prints them (`{dist:500 m name:"run"}`). A field of a mixed object
is any-typed at run time; its arithmetic has a text since card text-arithmetic (casts.rs: Kind::Data reads dynamically).
`q = p` and `ys = xs` copy the signatures (maps and lists are values). Recursion with quantities stays a loud error
(card static-units, Later). Since card reflection step 5 the text lives in the entry `units` of the module's one
`warp.meta` section (src/meta_section.rs), byte for byte as it was in `warp.units`; the section name below is history.

## Stage 8 done (card units-dynamic, 2026-10-09): units known only at run time
The dynamic struct, written in warp: lib/units.warp's `class Quantity {amount; dims; unit; scale}`. `quantity("5 km")`
reads text (a decimal stays exact: "2.5" is 5/2), `quantity(5, u)` takes a unit given as data. The amount is in SI
base units, dims packs the powers of length, mass and time as length + 256·mass + 65536·time (so `*` adds them), and
unit/scale is the display unit. `+ - < > ==` need equal dims, else "DimensionError: cannot add 3m and 2s"; `*` and `/`
combine (`m·m` shows `m²`, `m/s`); `q.to("m/s")` converts, `q.in("m")` gives a plain number. Units: mm cm m km inch ft
yd mi, mg g kg lb, ms s min h, mph, with powers (`m²`, `s^2`), products (`kg·m`) and one `/`.
- Loading (modules.rs): implicit when the program calls `quantity` and defines none, so hello world carries none of it.
  A module with classes is put in front of the program before class_methods (with_implicit_class_modules, needed
  definitions only), so `quantity(…) + …` dispatches to Quantity's operators. An explicit `use units` is replaced by it.
- Static units stay as they were: `5 km` written in the program is checked at compile time and has no struct.
- Fixed on the way: `t as number` of a run-time text was the text itself (now text_as_number: an Int when whole, else
  a Float); a function's result or an operation of instances counts as an instance for operators (class_methods.rs
  returned_classes); a parameter named like an instance variable is its own value.
- Open (cards): param-text-witness (`str(q)` of a parameter `q:Quantity` prints the record, the library uses
  quantity_text), instance-result-text (`"\(q.to("m"))"` prints the record; assign it first), number-field-float.
  A non-terminating ratio shows as a fraction (`151/15km/h`).
- Mixed (card units-mixed): a unit written in the program meeting a run-time quantity becomes one, on either side:
  `quantity("5 km") + 1 m`, `2 km < q`, `1 min == q` (class_methods.rs with_run_time_units, units.rs
  as_run_time_quantity: `5 m/s` is `quantity(5, "m/s")`). Static units alone stay static.
- ± (card plus-minus-units): `5 m ± 1 cm` the program needs at run time is a Quantity whose amount is a ± value
  (notes/plus_minus.md).
- Conversions and texts (card quantity-falls): `q as km/h` and `q in m` of a run-time quantity are `q.to("km/h")`, and
  `"v: " + q` is `"v: " + str(q)` (class_methods.rs with_operator_calls; units.rs conversion is the one reading of
  `as`/`in`). `w = q as m` and `q.to(…)` stay instances of Quantity (instance_classes, operand_class).

## Units in texts, imperial units (card units-text, 2026-10-09)
- `"total: " + (total as km)` and `(v in m/s)` join a text in the conversion's unit (static_units.rs: a parenthesized
  expression keeps its signature, the text arm reads conversion_target).
- src/units.rs UNITS: inch ft yd mi, lb. Factors count the smallest step of a dimension, so length counts 0.1 mm and
  mass 10 µg (1 ft = 3048, 1 lb = 45_359_237); time stays ms (milliseconds() reads the factor directly). The inch is
  `inch` (`inches`): `in` is the word of `x in xs` and `100 cm in m`.
- UNIT_ALIASES: `mph` is `mi/h`, rewritten by lower_unit_aliases before any units pass (the parser binds it to the
  amount like a unit: names_unit); a program defining mph keeps its own.
- `60 mi/h as km/h` parses `(60 mi/h as km)/h`; lower_unit_words regroups the whole unit after `as` (bottom-up, so
  `as g*cm/s²` too) when it is a unit expression and the program defines no unit name (card compound-unit).
