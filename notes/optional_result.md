# Optional results (decision optional-result, card optional-result-warning)

A function that can answer ø declares `T?`. A plain `: T` that can answer ø warns, and its result is then read as `T?`
so the ø reaches the caller unchanged, in every lane:

    def half(x: int): int { if x < 0 { return ø }; x / 2 }
    warning: half can answer ø; declare `: int?`     (the fix offer adds the ?)
    half(-1)  # ø

"Can answer ø", read from the body as written:
- `return ø` or a bare `return` (not inside nested definitions or lambdas)
- a last value that is ø, an `if` without `else`, a loop (`while … do`, `for`/`each … in`), a choice
  (`if/else`, `c ? a : b`) with such a branch, or a variable the body assigns such a value (`found = ø; …; found`)

An optional result of an `if` without `else` or of a loop is ø when nothing gave a value, not the kind's zero.
`any` results never warn.

Where: Rust `src/lowering/types/type_name_matching.rs` (optional_when_answering_empty, can_answer_empty, as_type);
warp `lib/compiler/definitions.warp` (result_held), warning row `optional_result` in `lib/compiler/warnings.data`.
emitter.warp holds a variable declared `Node?` or `Class?` as the node it is.

Not yet: ø flowing in through a call of a `T?` function.
