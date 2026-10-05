# An unknown word next to a value: `cube 3` (P51, P62)

Rule (wiki/charged.md, section 1 "Unknown words"): in code a word on its own is a symbol, a word applied to an
argument is the error; brackets change nothing; data is marked with `data …` (the `quote` prefix is dropped) or comes
from a data file.

- `WasmGcEmitter::unknown_word_error` (src/wasm_emitter/list_emitter.rs): a list of atoms (spaced, `(…)` or `[…]`) with a
  value and a word that names nothing (no variable, function, type, unit, keyword) is the error
  "undefined: cube in `cube 3`; define cube, or write `data cube 3` for data".
- Data contexts: `emit_quoted` (`data …`) and the value of an object entry (`emit_default_key`) set
  `data_context`, in which unknown words stay words.
- Not judged (unchanged): all-word lists (`hello world`, `[red green blue]`) and lists with operators (`foo x = 3`
  names its undefined variable itself). Comma tuples `(frobnicate, 3)` stay data.
- samples/data_structures.wasp: `mixed = data [1 "two" three 4.0]`.
Tests: tests/welcoming/test_unknown_word_error.rs.

## Refinement (user, 2026-10-05, P62/P63)
A word alone is a symbol (`[red green]`), with a near-miss got-it warning when it is one or two letters from a defined name
(`pirnt` → "did you mean print?", topic `near-miss`, WasmGcEmitter::warn_near_miss); a word applied to an argument (a
value or a defined name: `cube x`) is the error, brackets change nothing; `data …` is the only data prefix (`quote` was
dropped; `block`/`code` name blocks, wiki/charged.md).
