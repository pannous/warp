# Fixer log

Running list of the small fixes done by the fixer session (branches fix-<topic>).

## 2026-10-03 fix-print-args
- `print(a, b)` and `print a, b` print their arguments joined by a space (Python), worth the joined text.
  The parser turns the comma list `[[print a], b]` into the call `print(a, b)` (wasp_parser.rs
  `print_call_with_several_arguments`); the emitter prints `join([a, b], " ")` (list_emitter.rs `printed_value`).
  tests/test_print_arguments.rs (CLI stdout + is!, also in the browser).
- `warp parse` shows closing brackets and the separators (`,` `;` `⏎`), so a merged statement is visible.
- Found, in todo.md: one-line statements separated by spaces merge into one list; text * int (proposed to the supervisor).

## 2026-10-03 fix-small
- analyzer's copy of `collect_assigned_names` removed (identical to library_words').
- `ages["alice"]` no longer hints `ages#("alice"+1)`: a quoted key counts no position (normalize.rs index_operator).
  tests/test_key_subscript_hint.rs.

## 2026-10-03 fix-text-as-int
- `x="12"; x as int` / `int(x)` / `xs#1 as int` parse the text at run time (list_ops.rs `text_as_int`: optional sign,
  decimal digits; else runtime error "invalid number"); was "not an int". tests/test_runtime_text_as_int.rs.
