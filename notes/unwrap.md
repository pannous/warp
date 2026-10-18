# Unwrap `x!` (wiki_features row 27, wiki/optional.md, Error.md)

Wiki: "optionals can be unwrapped with a trailing `!` which throws if the value is missing or erroneous" (Swift).
By position (D2): `f!` after a function or method mutates, `{…}!` evaluates a block, a plain variable `x!` unwraps.

- `x!` is x when x is neither ø nor an Error.
- ø: the Error value "unwrapped ø" (`UNWRAPPED_EMPTY`, src/errors/run_time.rs). Errors are values (Decided #1): no throw,
  the error is raised where the value is used as a number.
- an Error: that Error, which propagates through arithmetic (`x!+1` is the Error, with its own message).

Lowering: src/lowering/definitions/mutation.rs turns a marked variable into the library word `UNWRAP`
(src/lowering/words/library_words.rs: `if hidden_value == ø then error "unwrapped ø" else hidden_value`). The parser
reads a glued `!` before an infix operator as a suffix, so `x!+1` parses. Error branches keep the other branch's
representation: notes/error_branch_kind.md.
Tests: tests/control/test_unwrap.rs, tests/control/test_unwrap_message.rs.
