# Fixer log

Running list of the small fixes done by the fixer session (branches fix-<topic>).

## 2026-10-03 fix-print-args
- `print(a, b)` and `print a, b` print their arguments joined by a space (Python), worth the joined text.
  The parser turns the comma list `[[print a], b]` into the call `print(a, b)` (wasp_parser.rs
  `print_call_with_several_arguments`); the emitter prints `join([a, b], " ")` (list_emitter.rs `printed_value`).
  tests/test_print_arguments.rs (CLI stdout + is!, also in the browser).
- `warp parse` shows closing brackets and the separators (`,` `;` `⏎`), so a merged statement is visible.
- Found, in todo.md: one-line statements separated by spaces merge into one list; text * int (proposed to the supervisor).

## 2026-10-03 fix-print-calls
- `print first [10, 5]` / `print(upper "ab")` / `print upper "ab", "c"`: words separated by spaces after print are ONE
  expression (wasp_parser.rs `grouped_list`, `print_arguments`); only commas separate print's arguments.
  Before, the call's arguments were flattened (`print(upper "a")` = print(upper, "a")). The parser keeps the
  canonical forms `[print expr]` (space) and `print(a, b)` (round); `print_arguments_of` reads both.
- `print "a"\n√9` already gives 3 on main (todo marked DONE).
