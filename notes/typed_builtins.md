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

## Next slices (candidates)
- A line form for words the emitter expands without a runtime function (count, map, filter, keys, …):
  MEMBER_SIGNATURES as prelude lines, without making them runtime words (list_emitter.rs is_runtime_word).
- Counting words: `size`, `len`, `length` as prelude aliases of count (COUNTING_WORDS has five readers).
- Number words: int and float overloads (abs, min/max, floor …) with their results, replacing the analyzer's arms.
- Blocker found: a `:=` definition with a list or map result type is lost (card list-result-type), so lib words
  written in warp cannot yet declare `: list`.
