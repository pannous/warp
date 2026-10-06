# Lazy ranges (card lazy-range, 2026-10-06)

User direction: big lists of numbers should be numbers internally; above a threshold lazy (an iterator) or symbolic, so
a GPU backend (P118: only for explicit `@gpu`) receives a descriptor instead of a list.

## What a range is now
- A range of plain bounds (integer literals or variables), `a..b` or `a to b`, is a descriptor (start, end) in the
  lowering pass `src/lowering/lazy_ranges.rs` (first of MEANING_PASSES):
  - `count r`, `#r`, `r.count`: `end - start`, at least 0 (constant for literal bounds)
  - `sum r`, `r.sum`: `n * (2 * start + n - 1) / 2` (exact, big integers included)
  - `r#i`: `start + i - 1` when `1 <= i <= n`, else the list's own index, so the error and its hint stay the same
  - `for x in r`, `r.map(f)`, `filter`, `each`, `fold`, `find`, `any`, `all`, `reduce` already looped over the bounds
- A range variable `xs = a..b` (in a statement list, not an object) is replaced by its range at every use when every
  use only reads it as above, nothing changes it (`xs#i = v`, `xs += …`), no function definition reads it as a global,
  its bound variables are not changed afterwards and no lambda parameter hides them. Otherwise it is a list, collected once.
- A range of literals up to 1000 numbers that is used as a value is still written out as its list at compile time
  (counting.rs `range_elements`, LITERAL_RANGE_MAX_LENGTH); a longer one is collected at run time into one int array
  (list_dispatch.rs typed lists), never a literal of 100000 nodes (that overflowed the compiler's stack).

## Ranges passed to functions (card range-value-run, 2026-10-06)
- A function defined once whose body only reads a parameter as a range (the same reads as a range variable, and it
  never changes it) is a range reader of that parameter. A call passing a range there (any bounds: `f(1..10^9)`,
  `f(2, 1..n)`, or a range variable replaced as above, `r = 1..n; f(r)`) calls a copy `f·range·<positions>(start, end, …)`
  whose body reads the parameter as the range `(p·start..p·end)`; the copy follows f's definition, f stays for list
  arguments. `a to b` passes b + 1 as its end. So the range travels as its two bounds (unboxed, no descriptor struct).
- Benchmark probes/range_argument_bench.sh (`def total(xs) = sum xs; total(1..n)`): n = 10^7 took 3.35 s before
  (collected 10^7 nodes, then summed them), 0.04 s after, flat in n.
- Transitive (card transitive-range): a parameter passed on only to range readers is read as a range too, worked out
  until nothing changes; a copy's body calls the copies of the readers it passes the range to
  (`g(ys) := sum ys; f(xs) := g(xs); f(1..10^7)`: 0.17 s, was a collected list like above). A recursive call is no
  reader, so `f(xs, n) := … f(xs, n - 1)` keeps its list.
- Not passed as bounds: a parameter that is changed, returned, printed or passed to a function that does not only
  read it, a function defined twice, a typed or defaulted parameter.

## Not yet
- No step (`1..10 step 2` has no syntax yet); a step joins the descriptor as its third field.
- A range printed, returned or stored in a structure is collected; a range value at run time (a descriptor struct the
  emitter knows, (start, end, step)) is what a GPU backend would receive (P118); passing to functions is solved by the
  bounds copies above without one.
