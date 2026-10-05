# Fixer log

Running list of the small fixes done by the fixer session (branches fix-<topic>).

## 2026-10-03 fix-print-args
- `print(a, b)` and `print a, b` print their arguments joined by a space (Python), worth the joined text.
  The parser turns the comma list `[[print a], b]` into the call `print(a, b)` (wasp_parser.rs
  `print_call_with_several_arguments`); the emitter prints `join([a, b], " ")` (list_emitter.rs `printed_value`).
  tests/text/test_print_arguments.rs (CLI stdout + is!, also in the browser).
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
  decimal digits; else runtime error "invalid number"); was "not an int". tests/text/test_runtime_text_as_int.rs.

## 2026-10-03 fix-declared-text
- `string x = "a"`, `x:string = "a"`, `text x = "a"` and a later `x = "c"`: a one-character text (parsed as a codepoint)
  assigned to a declared text is that text (analyzer.rs assignment_mismatch + lower_declarations_among).
  tests/text/test_declared_text_one_character.rs.

## 2026-10-03 fix-spaced-required
- `class person{name! email?}`: a `!` glued to its name and followed by a space is the suffix (required field / evaluate
  / mutate mark) even when an operand follows (wasp_parser.rs try_parse_evaluate_bang); `a ! b` spaced is unchanged.
  tests/parser/test_spaced_required_fields.rs.
- `while i<n {i++}` already works on main (todo marked DONE).

## 2026-10-03 fix-constant-text
- `str(1+2)`, `"" + (1+2)`, `"a" + 2*3`: an Int constant expression converts by its value ("3", "a6"), not its source
  (emit_cast "string": the source-text branch is for data and names only). Float/real constants still serialize (todo).
  tests/text/test_constant_expression_text.rs.

## 2026-10-03 fix-wasm-target
- A package tool build without the wasm32-wasip1 standard library reports "missing rust target wasm32-wasip1; fix:
  rustup target add wasm32-wasip1" (package_tools.rs build_from_source, rustc's "target may not be installed" note),
  user decision relayed by BOSS 2026-10-03. Checked rustc's wording with an uninstalled target; no test edits.

## 2026-10-03 fix-one-line-statements
- `print a    print b` (a print word or print(…) call after the first print of a space list) is the error "two statements
  on one line? separate them with `;` or a newline" (user decision). tests/parser/test_one_line_statements.rs.
- notes/open_decisions.md records this and the text * number decision (repeat + got-it warning, an assumption).

## 2026-10-03 fix-text-repeat
- text * int repeats (`"ab"*2` → "abab", `"5"*3` → "555"), educate_once "got it" warning naming `n times text`, and
  `int("5")*3` for a digit text (user decision "Python repeat"). `n times "ab"` / `n times g` repeat (parser marker
  `times·text`, a non-text is an error naming `n times [x]`). text * float/text stay type errors.
  tests/text/test_text_repeat.rs; approved edits of test_print_type_error, test_footguns, test_welcoming_sugar in their own commit.

## 2026-10-04 fix-ignored (the #[ignore = "next"/"soon"] sweep)
- Python unpacking (tests/probe_destructuring.rs, all 15 "next" probes now pass): `a, b = xs` of a list, text or
  `(1, 2)` unpacks by position (tuple_emitter.rs emit_unpacking; another count is the runtime error "wrong number of
  values"); `a, *rest = …` / `*init, last` / `a, *mid, z` (parser: `*name` → the starred symbol, tuples.rs STARRED;
  node_slice for the rest); `[a, b] = v`, `(a, b) = 1, 2`, nested `(a, b), c = …` (hidden `unpacked·i` names).
- `f(int x, float y)` / `fun f(int a, int b){…}`: every comma argument may be a typed parameter (typed_parameters).
- Exact reals join texts symbolically in constant programs (real.rs Value::Text): `"f" + sqrt(2)` → "f√2", `π/2 as
  string` → "π/2" (user: "√2 if we preserve that information symbolically").
- test_types "soon" tests un-ignored where they pass (their assertions are comments): test_typed_functions,
  test_empty_typed_functions, test_polymorphism, test_polymorphism2. Still ignored, need real features: return-type
  annotations `def f(x):float := …` (undefined function f), overloading by parameter type (test_polymorphism3), and
  test_function_argument_cast (C-style `float addi(int x,int y){…}` return-typed definitions, int parameters
  truncating float arguments).

## 2026-10-05 fix-ignored-2
- Indexing ø (`x=ø; x#1`, `x[0]`) was a raw "wasm trap: cast failure" a `try` could not catch; ø is the empty list,
  so emit_list_walk fails index_out_of_range (tests/control/test_try_empty_index.rs).
- A non-Int list element read as a number inside `try` (`x=[1,"ab"]; try -x#2 else 7`) was a cast trap: list_at reads
  its element through get_int_value (code point of a character, else not_an_int); the runtime errors and getters are
  emitted before the list ops now (tests/control/test_try_list_index.rs).

## 2026-10-05 fix-uniscript-entities
- `\alpha` / `\:infinity` uniscript entities in code (src/uniscript_entities.rs, ~150 LaTeX and English names, Greek,
  sets, logic, relations): expanded to their character before parsing, outside texts and comments; an unknown
  `\name` is the loud "unknown entity \name" (tests/parser/test_uniscript_entities.rs). The full table and `<:…>`
  blocks stay with the uniscript package.

## 2026-10-05 p22-like-error
- P22 was already the behaviour on main (a known other type is an error teaching `pic like photo`); pinned the
  missing-field case (tests/operators/test_like_known_type_mismatch.rs). Ad hoc names (`pic{…}` with no class pic) keep
  their field warnings: `like` needs declared types.

## 2026-10-05 p26-libm-pure
- P26: libm calls carry no FFI effect; a Libm capability (granted to eval and untrusted code) keeps them imported
  (tests/functions/test_libm_pure.rs; two pinned tests edited in their own commit).

## 2026-10-05 p49-float-to-int-param
- P49: a fractional literal or a variable holding one passed to a declared int parameter is the compile error
  "2.2 is no int: write 2.2 as int" (analyzer infer_parameters_from_calls); 2.0 passes (no digits lost, assumption).
  test_function_argument_cast edited (approved) and un-ignored.

## 2026-10-05 fix-try-raise
- `try f x else y`: the guarded part may be a braceless call (was "`try` needs an `else`" after the first word)
  (tests/control/test_try_braceless_call.rs).
