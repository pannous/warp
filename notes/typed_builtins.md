# Typed builtins (card typed-builtins)

User wish (2026-10-18, via the supervisor): every built-in word gets fully typed overloads in lib warp, with declared
result types, so static dispatch picks the overload and the Rust kind tables go (inference.rs builtin_word_kind and
library_call_kind, library_signatures.rs MEMBER_SIGNATURES, counting.rs COUNTING_WORDS) together with the run-time
kind switches they stand for. Done in slices per word family, each with its net src/ delta.

## Mechanism (slice 1: reverse, sort, slice)
- A word of several `from warp` lines in lib/prelude.warp is overloaded by its first parameter's type:
  `reverse(xs: list): list from warp` and `reverse(t: text): text from warp`.
- library_words.rs call_result_kind: overloads that agree give their shared result without looking at the argument;
  else the argument's kind picks the line (a character counts as a text); a kind no line is typed for (a Node of
  run-time kind, a range) gives Kind::Empty, a Node decided at run time. The argument is inferred only then.
- library_signatures.rs library_types lists every overload's type (the typed-redefinition refusal names each).
- `reverse.signature` shows the first line, `(xs:list) -> list` (was the untyped `(xs)`).

## Landed (2026-10-18)
- Slice 1 (reverse, sort, slice): overloaded `from warp` lines. An argument no line is typed for takes the first
  line's result (a list's); only a Node of run-time kind gives a Node (slice-twin: the analyzer twins agree on
  `b = a[0:2]`, words.warp call_kinds says list).
- Counting: `alias size/len/length = count` in the prelude; COUNTING_WORDS is gone, is_counting_word reads the aliases.
- `from emitter` lines (warp-self-analyzer): builtins the emitter expands in place declare signature and result
  (count, byte_size, type, print, puts/puti/putl/putf, fetch); library_words::emitted_call_kind replaced five
  builtin_word_kind arms. A `number` or `bool` result stays undecided (Int would mis-type `number("2.5")` and drop
  any()'s yes/no).
- Members: MEMBER_SIGNATURES became 15 `from emitter` lines (map, filter, …, number, string), read last by
  library_signature.
- Number words have no analyzer table: abs lowers to ‖x‖, min/max inline, floor/round/ceil go through lib/math.warp.

## Left
- library_call_kind's arms whose result follows an argument (field_with/instance_copy: the object's kind, map_get_or:
  a value of the map, list_become/map_of_entry: the place's): need a type variable in a line
  (`field_with(object: T, name, value): T`), a design step of its own.
- PRINT_SIGNATURE `(x)` beside print's `(values...)` line: dropping it makes print variadic for library_arity.
- Blocker: a `:=` definition with a list or map result type is lost (card list-result-type), so lib words written in
  warp cannot yet declare `: list`.
