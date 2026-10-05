# An unknown word next to a value: `cube 3` (P51, P62)

User decisions 2026-10-05: P51 "should obviously be an error unless we're in a clear data context"; P62: the data
contexts in a program are a `quote`/`data` prefix and the values of an object literal (`{shape: cube 3}`), plus data
mode (`parse_data`, `warp data`). Brackets are no data context: `(cube 3)` and `[cube 3]` in code are code.

- `WasmGcEmitter::unknown_word_error` (src/wasm_emitter/list_emitter.rs): a list of atoms (spaced, `(…)` or `[…]`) with a
  value and a word that names nothing (no variable, function, type, unit, keyword) is the error
  "undefined: cube in `cube 3`; define cube, or write `quote cube 3` for data".
- Data contexts: `emit_quoted` (`quote …`, `data …`) and the value of an object entry (`emit_default_key`) set
  `data_context`, in which unknown words stay words.
- Not judged (unchanged): all-word lists (`hello world`, `[red green blue]`) and lists with operators (`foo x = 3`
  names its undefined variable itself). Comma tuples `(frobnicate, 3)` stay data.
- samples/data_structures.wasp: `mixed = quote [1 "two" three 4.0]`.
Tests: tests/welcoming/test_unknown_word_error.rs.
