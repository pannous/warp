# Compiler refactor

Roadmap card: compiler-own-code (board)

The compiler's own code, made to read like the language it compiles. The user fills in the steps.

| # | Card | Step |
|---|------|------|
| 1 | code-type | `type Code = list of byte` for wasm bytes: the emitter writes them as plain `list` today (158 emitter functions return `list`, e.g. `fresh_int_code(code: list): list`, `entry_code(name_code: list, value_code: list): list`); naming the type says what the list holds and lets a wrong list be refused (wiki/alias.md shows the alias) |
