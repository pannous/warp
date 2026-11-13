# Run speed of warp programs

What a program's own loops cost on the native warp (wasmtime), measured with `/usr/bin/time -p warp --no-window <file>`;
compile time (~0.25 s) is in each figure. Compile speed: notes/compile_speed.md.

## A lib word's call costs nothing measurable (2026-11-12, card lib-inlining)
The card asked for compile-time inlining of small lib words because a prelude call was 12% slower in a loop (Rust-era
figure, card sum-list). Measured again, 400 M iterations of `s = s + …`:

| body | seconds |
|---|---|
| `min(i, 7)` (prelude) | 5.33–5.41 |
| `my_min(i, 7)`, the program's own | 5.43–5.48 |
| `if i < 7 then i else 7`, written inline | 5.24–5.30 |

Within 1–3%, about the noise: moving a word to lib/ costs no speed, so no inliner is built. What did cost was a word's
body: `sum(list)` was a fold of the copied rest through a `+` closure (below).

## Summing a list (card sum-list-fold)
`sum(xs)` of 8 ints, 30 M times:

| body | seconds |
|---|---|
| `fold(l[1 ..], l#1, +)` (sum before) | 13.4 |
| `total = l#1; for x in l[2 ..] { … }` | 11.9 |
| `total = 0; for x in l { … }` | 6.1 |
| `total = l#1; for k in 2...count(l) { total = total + l#k }` | 4.9 |
| `l[2 ..]` alone | 2.2 |

A slice copies; `for x in` a list walks slower than indexing it. sum is the index loop now: 8.5–8.8 s, not the
4.9 s of the program's own loop (its `list or number` parameter and the range and number tests before the loop are
the rest; not measured apart yet).

## Exact numbers in a loop
`s = 0.0; for i in 1..n { s = s + sqrt(i) }` keeps an exact radical sum (`.0` is exact, roots stay exact): 1000 steps
~55 s, 2000 past a minute. `as float` puts such a loop on floats (wiki/Footguns.md); the warning decision
exact-decimal-big promises is missing (card loop-summing).

## Constant tables cost milliseconds (2026-11-13, card constant-data)
The Unicode case tables (core/runtime/case_tables.warp, 9636 ints in three texts) are parsed when first used:
`upper(…)` builds its table lazily, `use case_table` builds all three when the module loads. Measured end to end:

| program | seconds |
|---|---|
| `print "straße"` | 0.07–0.09 |
| `print upper("straße")` (builtin, one table) | 0.08 |
| `use case_table; print "x"` (module linked, three tables built) | 0.09–0.10 |
| `use case_tables`, all three tables parsed once more | 0.11 |

About 15 ms for all three: a bytes or int-array literal emitted as a data segment (array.new_data) would save that,
for core emitter work, so none is built until a table is large enough to show in a program's time.
