# An unknown word applied to a value: `cube 3`

Decided #7 makes `cube(3)` (a call written without a space) the error `undefined function: cube`. The spaced form
`cube 3` stayed silent data (notes/unresolved_call_survey.md, rule 3: `print 3` is one way to write call-shaped data).
Now it stays data but warns once per topic (got-it topic `unknown-word`, notes/welcoming.md):

    warning: cube is no function: `cube 3` is data, not a call (define cube(x) := … to call it) … fix: [cube 3]

- Where: `WasmGcEmitter::warn_unknown_prefix_word` (src/wasm_emitter/list_emitter.rs), at the data fallthrough of
  `emit_list_node`, after `reject_unresolved_call`. Under `use strict` the warning is an error.
- When: a spaced list (no brackets) whose head is a word that is no variable, global, function, keyword, type or
  library word (`resolves_call`), with at least one argument that is a value (not a bare word, not a `{…}` block).
- Quiet: `hello world` (words only), `person{name:"Joe"}`, `[cube 3]` (written data), a defined `cube`.
- Tests: tests/welcoming/test_unknown_prefix_word.rs. Open decision P51: a loud error instead of the warning.
