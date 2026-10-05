# Unwrap `x!` (wiki_features row 27, wiki/optional.md, Error.md): plan, not implemented

Wiki: "optionals can be unwrapped with a trailing `!` which throws if the value is missing or erroneous" (Swift).
D2 ("by position", decided): `f!` after a function or method mutates, `{…}!` evaluates a block. A plain variable
`x!` is left for unwrap; today the parser marks the name (`mutation::marked`) and mutation.rs drops an unconsumed mark,
so `x!` is just `x` (silently, also for ø).

## Semantics (proposed)
- `x!` is x when x is neither ø nor an Error.
- ø: the Error value "x! is ø" (no throw: errors are values, Decided #1; catch/raise are row 28 / T2).
- an Error: that Error, which then propagates through arithmetic (`x!+1` is the Error).

## Lowering (cheap part)
mutation.rs, where an unconsumed mark is dropped (`Node::Meta { .. } if is_marked`, `strip_marks`): lower a marked
variable (not a function name: `f!` evaluates) to `if x == ø then error("x! is ø") else x`. The parser also needs
`x!+1`: a glued `!` followed by an infix operator is a suffix (try_parse_evaluate_bang refuses it as an operand
follows).

## Blocker (why it is not done)
`(if x == ø then error("x! is ø") else x) + 1` with x = 3 is `wasm trap: cast failure`. `error(…)` is typed Text
(text_builtins.rs `(ERROR, 1, Kind::Text)`), so `branches_kind` makes the if a Node (Text and Int), and the arithmetic
then casts the Int Node wrongly. Needed first: Kind::Error as a branch kind that keeps the other branch's
representation for the value path and is checked at use (`is_error` before the cast), in analyzer::branches_kind and
the emitter's if. That changes the typing of every if with an error branch: a task of its own.
