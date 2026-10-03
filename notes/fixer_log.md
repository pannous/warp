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

## 2026-10-03 fix-small
- analyzer's copy of `collect_assigned_names` removed (identical to library_words').
- `ages["alice"]` no longer hints `ages#("alice"+1)`: a quoted key counts no position (normalize.rs index_operator).
  tests/lists/test_key_subscript_hint.rs.

## 2026-10-03 fix-text-as-int
- `x="12"; x as int` / `int(x)` / `xs#1 as int` parse the text at run time (list_ops.rs `text_as_int`: optional sign,
  decimal digits; else runtime error "invalid number"); was "not an int". tests/test_runtime_text_as_int.rs.

## 2026-10-03 fix-declared-text
- `string x = "a"`, `x:string = "a"`, `text x = "a"` and a later `x = "c"`: a one-character text (parsed as a codepoint)
  assigned to a declared text is that text (analyzer.rs assignment_mismatch + lower_declarations_among).
  tests/test_declared_text_one_character.rs.

## 2026-10-03 fix-spaced-required
- `class person{name! email?}`: a `!` glued to its name and followed by a space is the suffix (required field / evaluate
  / mutate mark) even when an operand follows (wasp_parser.rs try_parse_evaluate_bang); `a ! b` spaced is unchanged.
  tests/test_spaced_required_fields.rs.
- `while i<n {i++}` already works on main (todo marked DONE).

## 2026-10-03 fix-constant-text
- `str(1+2)`, `"" + (1+2)`, `"a" + 2*3`: an Int constant expression converts by its value ("3", "a6"), not its source
  (emit_cast "string": the source-text branch is for data and names only). Float/real constants still serialize (todo).
  tests/test_constant_expression_text.rs.

## 2026-10-03 fix-wasm-target
- A package tool build without the wasm32-wasip1 standard library reports "missing rust target wasm32-wasip1; fix:
  rustup target add wasm32-wasip1" (package_tools.rs build_from_source, rustc's "target may not be installed" note),
  user decision relayed by BOSS 2026-10-03. Checked rustc's wording with an uninstalled target; no test edits.

## 2026-10-03 fix-one-line-statements
- `print a    print b` (a print word or print(…) call after the first print of a space list) is the error "two statements
  on one line? separate them with `;` or a newline" (user decision). tests/test_one_line_statements.rs.
- notes/open_decisions.md records this and the text * number decision (repeat + got-it warning, an assumption).

## 2026-10-03 fix-text-repeat
- text * int repeats (`"ab"*2` → "abab", `"5"*3` → "555"), educate_once "got it" warning naming `n times text`, and
  `int("5")*3` for a digit text (user decision "Python repeat"). `n times "ab"` / `n times g` repeat (parser marker
  `times·text`, a non-text is an error naming `n times [x]`). text * float/text stay type errors.
  tests/test_text_repeat.rs; approved edits of test_print_type_error, probe_footguns, test_welcoming_sugar in their own commit.
