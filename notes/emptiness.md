# Emptiness and absence (card empty-is-absence, decision empty-is-not-missing)

ø is absence, not emptiness: an empty list, text or map is a value that is there.

- `[] == ø`, `"" == ø`, `[] is ø`, `[[]].contains(ø)`: no. `x == ø` / `x is ø` / `x != ø` ask whether x is ø itself
  (emitted as a null test, like `===`; Rust lane `equality.rs nothing_identity`, emitter.warp `identity_with_nothing`).
- Emptiness is asked loosely: `x is empty`, `if no x`, `if not x`. Has items is plain `if x`.
- `no <operand>` in a condition is `not` (lookahead.rs / parser.warp `no_asks_emptiness`); elsewhere `no` stays the
  word it was (not before `{`, `:` or an operator word).
- `∅` is an alias of ø (got-it note "I meant: ø").
- `[]` keeps its EMPTY_LIST_KIND at run time (notes/run_time_empty.md); equality no longer folds it into Empty.

Not yet: `{}` in code is still ø (card code-empty: it becomes an empty map).
