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

## Not yet
- No step (`1..10 step 2` has no syntax yet); a step joins the descriptor as its third field.
- A range passed to a function or printed is collected; a range value at run time (a descriptor struct the emitter
  knows) would let `f(1..10^9)` stay lazy, and is what a GPU backend would receive (card linear-memory, P118).
