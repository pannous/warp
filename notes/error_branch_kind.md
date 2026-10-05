# An `error(…)` branch is the bottom kind; unwrap `x!`

- `analyzer::raises_error`: an if-branch that is `error(…)` (or a block ending in it) never gives its if a value, so
  the other branch decides the kind: `if c then error("…") else 3` is an Int (it was a Node, and `(…)+1` a cast
  failure). else-if chains and switch defaults follow, as they lower to ifs.
- `WasmGcEmitter::emit_raised_error`: `error(…)` emitted as a number (Int or Float) fails the run with its message
  through `returned_error`, the path `return error(…)` from a function of numbers already used; `try` catches it.
- Where a Node is wanted the Error stays a value (Decided #1, `r = f(-1); if r failed …`): an if of Node kind with
  an error branch gives the Error value, as before.
- `x!` (D2 by position: after a plain variable): mutation.rs turns a `!` mark that no mutation consumed into the
  library word `unwrap(x)` = `if x == ø then error("unwrapped ø") else x`. An Error value stays that Error.
  `x!+1`: a `!` glued to its name and followed by an infix operator is a suffix (INFIX_AFTER_BANG).
- An Error value used as a number in Node arithmetic (node_add …) fails with its own message: it is left in the
  exported `trap_detail` global and `returned_error` raises it (the runner reads the Error's message), so
  `x:int?=ø; x!+1` is "unwrapped ø" and `x = f(-1); x + 1` names f's error, never "not a number".
Tests: tests/control/test_error_branch_kind.rs, tests/control/test_unwrap.rs.
