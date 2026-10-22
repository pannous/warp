# Optional results (decision optional-result, card optional-result-warning)

A function that can answer ø declares `T?`. A plain `: T` that can answer ø warns, and its result is then read as `T?`
so the ø reaches the caller unchanged, in every lane:

    def half(x: int): int { if x < 0 { return ø }; x / 2 }
    warning: half can answer ø; declare `: int?`     (the fix offer adds the ?)
    half(-1)  # ø

"Can answer ø", read from the body as written:
- a bare `return`, or `return v` with a v that can be ø as below (not inside nested definitions or lambdas)
- a last value that is ø, an `if` without `else`, a loop (`while … do`, `for`/`each … in`), a choice
  (`if/else`, `c ? a : b`) with such a branch, a variable the body assigns such a value (`found = ø; …; found`), or a call of a
  function whose result can be ø (declared `T?`, or `T` by these rules, found for the whole program first).
  A call's ø kept in a variable does not count: it is mostly tested before it is given back
  (`x = f(); if x == ø …; x`), and without narrowing the rule would warn there falsely.
- `T?`, `any` and `Node` hold ø already; `op.return` (a member) is no return.

A list result is the exception (user 2026-10-22, "ø can autocast to []", card list-result-autocast): a function
declared `: list` (`ints`, `list of T`) that can answer ø gives [] instead, with no warning
(`(body as list?) ?? []` in Rust, `body ?? []` in definitions.warp result_held).

An optional result of an `if` without `else` or of a loop is ø when nothing gave a value, not the kind's zero.
`any` results never warn.

Where: Rust `src/lowering/types/type_name_matching.rs` (optional_when_answering_empty, can_answer_empty, as_type);
warp `lib/compiler/definitions.warp` (result_held), warning row `optional_result` in `lib/compiler/warnings.data`.
emitter.warp holds a variable declared `Node?` or `Class?` as the node it is, one declared `ints` or `list of T` as a list.

