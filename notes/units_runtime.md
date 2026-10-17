# Quantities at run time

Naming (decisions.md quantity-naming): avoid the bare word "dimension", it is ambiguous (a 28 × 28 px image has two
axes, each of them a length). Quantity (ISO 80000 kind of quantity) for length, speed, density; axis for width/height;
rank for the number of axes; shape for 28 × 28 px; `dimension` only an alias with a hint.

Two mechanisms:
- **Static units** (src/numbers/units/static_units.rs and static_units/): a unit written in the program is part of the
  static type, the run-time value is only the amount, an exact number (F#-style units of measure, no GC struct, loops and
  lists cost nothing extra). Started as stage 1 (variables, loops, branches); stages 2-7 added functions, output,
  lists, compiled results, objects.
- **Stage 8, run-time units**: lib/units.warp's `class Quantity` for units known only at run time (`quantity("5 km")`).

## Static units
Every program with quantities takes this one path: unit signatures are checked at compile time, amounts run as exact
whole steps of their quantity (`1 km/h + 1 m/s` → 23/18 m/s). Units bind only after a number (`x m` is the
DimensionError "a unit binds only after a number, write x * 1 m"); the style hint prefers `3m` over `3 m`.
- Signatures: `*` `/` combine, `+ - == <` and the branches of an if need equal signatures (DimensionError), a variable
  keeps one signature, `q^2` with a written whole power works like `q²`, `√a` halves the powers (odd power: error).
- Temperatures run as kelvin from absolute zero: `40°C` is 313.15, shown back as 40°C; a sum or negation of temperatures
  counts one zero (`20°C + 5°C` is 25°C), a sum of °C and °F fails ("one unit only").
- Compile-time constants (constant sleep durations, `time < 24h`, main-level constants `x = 2 km; if x > 1500 m`, a
  database column's SI amount) fold through `static_units::constant`, the same inference. A tolerance with a unit is
  always the run-time `quantity(5 ± 1/100, "m")` (notes/plus_minus.md).
- Functions (function_units.rs): a call with quantity arguments (or a body with unit literals) is specialised per unit
  signature of its arguments (`speed·u0`), its result signature inferred from the body; `speed(10 km, 2 h)` → 5 km/h,
  `return q` carries the signature and all returns of a function must agree. Field names after `.` are no units (`q.s`);
  a unit word heading a call or command is no unit (`min(n, 3)`, `use m`).
- Recursion: a recursive call into the specialisation being inferred gives `Stop::Recursive`, the if/else takes the base
  case's signature, and the body is inferred a second time with recursive calls returning that result, which must come
  out the same (`x * p(n-1, x)` with a base case `1`: "the branches give a plain number and m"). Recursion that changes
  the arguments' units (`f(n-1, x*x)`) stays the loud error. Test: tests/numbers/test_recursive_units.rs.
- Output: `print q`, `"text " + q`, `"${d}"` (interpolation lowers to `text_form(d)`) show `str(amount / unit) + " km"`
  at run time (a fraction with a finite decimal prints as the decimal: 2.5 km/h); `q as km` / `q in km` check the
  quantity and choose the unit shown, a final conversion decides how the result reads. `"total: " + (total as km)`
  joins in the conversion's unit (a parenthesized expression keeps its signature). `q.serialize()` gives `"4 m"`.
- Lists (list_units.rs): a list's `Signature` is its elements' powers with a depth (a matrix 2, a plain list stays
  plain), so any list-valued expression carries it: `[1 m, 2 m]`, `[3, 4] cm`, `xs cm` (`map` at run time), `xs * 2`,
  `f([3, 4] cm)`. `[1 m, 2 s]` is a DimensionError. Index, `for`, `.map`, `sum`/`max`/`min`/`first`/`last` peel one level,
  `count` is plain, `x in m` converts each element, `xs + [5 cm]` and `xs.add(q)`/`xs.push(q)` append (checked),
  `xs + 1 cm` is a compile error, `type(xs)` is `list of length`, `print`/`str` show `[3cm 4cm]`.
- Objects (unit_fields.rs): a variable holding an object literal with quantity fields notes one signature per field
  (`p = {dist: 0 m}`), so `p.dist += 250 m` checks and computes; `print p`, `"${p}"`, `text_form(p)` build the text at
  run time (`{dist:500 m name:"run"}`). `q = p` and `ys = xs` copy the signatures (maps and lists are values).
  `x:any = q` keeps the signature, `x:km = q` checks it (annotated code is strict) and x is then a plain number.
  `dist: 5 m` is data, not a lazy block (blocks.rs is_computed).
- Compiled results: a module whose result is a quantity names its unit in the entry `units` of the module's `warp.meta`
  section (src/formats/meta_section.rs; `km:1 h:-1`, an object `dist=m:1;t=s:1`), appended by the pipeline for eval and
  `compile` alike; wasm_reader, running a `.wasm` file and the playground's run_module read it back, so `warp compile …`
  then `warp out.wasm` prints `500 m`.
- Quantities as types (`DIMENSION_NAMES`, `dimension_signature`): `3 cm is length` → yes, `4 m² is length²`,
  `20°C is temperature`; a declared `h: length = 2 inch` accepts any unit of the quantity and keeps the written one,
  `w: mass = 3 m` is a DimensionError. Derived names come from lib/units.warp (`quantity speed m/s`: `type(5 m/s)` is
  speed). The time quantity is `duration`, a subtype of `time`: `3 s is duration` and `3 s is time` hold; a value
  without units keeps the time module's test (`now is time`). A program naming `length` itself keeps its variable.
  Test: tests/numbers/test_dimension_types.rs.
- A range with a unit (`1900 - 2000 AD`) is a subtraction: the range tests are ignored as `cleanup-edge` (card
  discuss-moving).
- Where a unit reaches the emitter uncomputed, the error says so (src/wasm_emitter/control/traps.rs):
  tests/numbers/test_units_at_run_time.rs pins that every such case fails loudly, none drops a unit.
- Open: `sum(xs cm)` (card parser-glues), a final unnamed object literal with a variable field (`d=3m; {dist: d}`
  prints dist:3, card final-unnamed).

## Unit definitions in lib/units.warp (decisions unit-definitions, unit-target, pixel-lengths)
Quantities and units are no Rust tables: lib/units.warp's `unit_definitions = data { … }` block names them, and
src/numbers/units/definitions.rs reads it once with its own small line reader (the parser asks `is_unit` to read
`201.168 m`, so it cannot parse the block that defines m):
- `SI quantity length meter`: a base quantity and its SI anchor, whose units take the SI prefixes (`prefix k := 10^3`,
  giga down to nano: every unit stays whole steps of its quantity in i64); a new base quantity also needs its
  `Dimension` variant. `quantity speed m/s` names the quantity of an expression of units; `quantity pixels px` and
  `quantity era AD` are quantities of their own.
- `unit m := 1 meter` its symbol, `unit cm := m/100`, `unit furlong := 201.168 m`: an amount of one known unit, which
  fixes quantity and factor; `unit °C as K := it + 273.15` counts from another zero, `it` the amount in the new unit.
- `unit mph := mi/h`, `unit N := kg*m/s²`, `unit L := dm³`, `unit ° := π/180`: several units or none, a word for its
  expression (expanded by units.rs lower_unit_aliases before any units pass; a program defining mph keeps its own).
  `name hour := h`: a long name a conversion target may use, singular or plural (`2 h in minutes`). The inch is `inch`:
  `in` is the word of `x in xs` and `100 cm in m`.
- `60 mi/h as km/h` parses `(60 mi/h as km)/h`; lower_unit_words regroups the whole unit after `as` (bottom-up, so
  `as g*cm/s²` too) when it is a unit expression and the program defines no unit name.
- No bridges between quantities: `cm as px` is a DimensionError ("multiply by an amount in dpi"); a length becomes px by
  arithmetic, `3 cm * display.density` (lib/prelude.warp: `display_dpi * 1dpi`, set by `display.density = 300 dpi`;
  warp-runtime system_values.rs keeps the set value, else the screen's (CoreGraphics, xrandr), else DEFAULT_DPI 96; the
  playground host.js keeps holder.displayDpi, else 96·devicePixelRatio).
- Each unit becomes whole steps of its quantity (amount × lcm of the quantity's denominators); only ratios matter.
- The block is also the run-time table: with_run_time_unit_tables replaces the statement by
  unit_names/unit_scales/unit_dimensions, so `quantity(1, "furlong")` works.
- A program's own units are unit lines too: `unit furlong := 201.168 m` at the start of a line. pipeline.rs
  lawful_program reads them before parsing (definitions::program_unit_lines, kept in context.program_unit_lines until
  the next program, so the result reads back), so `3 furlong` binds like `3 m`; units::without_unit_lines then drops
  them, an unreadable one (`unit x := 3 foo`) is an error. A plain `furlong := 201.168 m` stays a constant.
- Open: px as the device pixel through display.dpi (Q1).
Tests: tests/numbers/test_unit_definitions.rs, test_pixel_unit.rs.

## Stage 8: units known only at run time (lib/units.warp)
`class Quantity {amount; dims; unit; scale}`. `quantity("5 km")` reads text (a decimal stays exact: "2.5" is 5/2),
`quantity(5, u)` takes a unit given as data. The amount is in SI base units, dims packs the powers of length, mass and
time as length + 256·mass + 65536·time (so `*` adds them), unit/scale is the display unit. `+ - < > ==` need equal dims,
else "DimensionError: cannot add 3m and 2s"; `*` and `/` combine (`m·m` shows `m²`, `m/s`); `q.to("m/s")` converts,
`q.in("m")` gives a plain number. A non-terminating ratio shows as a fraction (`1/7km/s`).
- Loading (src/modules/implicit_modules.rs): implicit when the program calls `quantity` and defines none, so hello world
  carries none of it. A module with classes is put in front of the program before class_methods (needed definitions
  only), so `quantity(…) + …` dispatches to Quantity's operators. An explicit `use units` is replaced by it.
- Mixed: a unit written in the program meeting a run-time quantity becomes one, on either side: `quantity("5 km") + 1 m`,
  `2 km < q`, `1 min == q` (class_methods.rs with_run_time_units, units.rs as_run_time_quantity: `5 m/s` is
  `quantity(5, "m/s")`). Static units alone stay static.
- ±: `5 m ± 1 cm` the program needs at run time is a Quantity whose amount is a ± value (notes/plus_minus.md).
- `q as km/h` and `q in m` of a run-time quantity are `q.to("km/h")`, `"v: " + q` is `"v: " + str(q)`
  (class_methods.rs with_operator_calls); `w = q as m` and `q.to(…)` stay instances of Quantity.
Test: tests/numbers/test_dynamic_units.rs.
