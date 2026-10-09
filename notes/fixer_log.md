# Fixer log

Running list of the small fixes done by the fixer session (branches fix-<topic>).

## 2026-10-03 fix-print-args
- `print(a, b)` and `print a, b` print their arguments joined by a space (Python), worth the joined text.
  The parser turns the comma list `[[print a], b]` into the call `print(a, b)` (warp_parser.rs
  `print_call_with_several_arguments`); the emitter prints `join([a, b], " ")` (list_emitter.rs `printed_value`).
  tests/text/test_print_arguments.rs (CLI stdout + is!, also in the browser).
- `warp parse` shows closing brackets and the separators (`,` `;` `⏎`), so a merged statement is visible.
- Found, in todo.md: one-line statements separated by spaces merge into one list; text * int (proposed to the supervisor).

## 2026-10-03 fix-print-calls
- `print first [10, 5]` / `print(upper "ab")` / `print upper "ab", "c"`: words separated by spaces after print are ONE
  expression (warp_parser.rs `grouped_list`, `print_arguments`); only commas separate print's arguments.
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
  / mutate mark) even when an operand follows (warp_parser.rs try_parse_evaluate_bang); `a ! b` spaced is unchanged.
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

## 2026-10-05 fix-count-in
- `count x in y` counts occurrences (items of a list, characters of a text, substrings of a text) instead of being
  read as count(x in y) = 1; `count bytes in t` is t.bytes. Own pass before the lambdas (library_words::lower_count_in).

## 2026-10-05 fix-try-raise
- `try f x else y`: the guarded part may be a braceless call (was "`try` needs an `else`" after the first word)
  (tests/control/test_try_braceless_call.rs).

## 2026-10-05 fix-infinity
- `∞` is the float infinity (Number::Inf, typed Float): `∞ > 1e300`, `-∞`, `1.0/0.0 == ∞`, `\:infinity`
  (tests/numbers/test_infinity.rs). P56 (4) default; ω stays the hyperreal.

## 2026-10-05 fix-hash-slices
- Slices: `xs[a..b]`, `xs[a:b]`, text and variable bounds already worked; new `xs#(a…b)` / `xs#(a..b)` 1-based
  slices (warp_parser hash_slice_bounds). Unparenthesized `xs#a..b` stays the range from the value xs#a (it works on
  main: `xs#1..6`), so it is no slice; queued as a question.

## 2026-10-05 fix-is-declaration
- `x is number 9` of a name assigned nowhere (and no parameter) declares it, `x:number = 9` (wiki Features.md,
  inventions.md); of a variable it stays the type-and-value test (lowering/type_tests.rs is_declaration).

## 2026-10-05 fix-raise
- `raise X` / `throw X` / `raise error("m")`: the builtin raise(X) fails the run through returned_error with X as its
  detail; `try` catches it, an Int if treats it as the failure branch (pipeline::returned_error_message). `catch` is
  not built: the wiki form (function-level `catch (no food){}` handlers) is queued as a question.

## 2026-10-05 decided-test-edits
- P55: test_while_nop_issue reads x after the loop, un-ignored. P59: four array tests edited and un-ignored (own commit);
  `x is 100 times [0]` written as the assignment `x = 100 times [0]` (assumption, `is` compares).

## 2026-10-05 p56-entities
- P56: entities are `\:name` only, in code and inside double-quoted texts (the text parser expands them); a bare
  `\alpha` is the error "a uniscript entity is written \:alpha"; unknown `\:name` is loud in code and texts.

## 2026-10-05 p49b-whole-float
- P49b: a whole float (2.0, y=3.0) passed to an int parameter is refused too ("2.0 is no int: write 2.0 as int").

## 2026-10-05 p58-hash-range-warning
- P58: `xs#a..b` stays the range from the value xs#a, with a warning naming the slice xs#(a..b) and the range (xs#a)..b
  (warp_parser hash_range_warning).

## 2026-10-05 p60-catch-except
- P60: `try {…} catch {…}`, `catch e {…}`, `try: … except: …`, `except E:` / `except E as e:` are synonyms of `try X else Y`
  (warp_parser FALLBACK_WORDS, parse_caught_name); using the caught name is a loud error for now.

## 2026-10-05 p61-is-teaches-be
- P61: `is` always compares; `x is v` with x unbound is the error "undefined variable: x; `is` compares, a definition is
  written `x be v`" (v as written: the parser keeps it as meta "compared with"); the `x is number 9` declaration undone.

## 2026-10-05 p44-atomic
- P44: `atomic xs = int[n]` is `shared xs = int[n]` (shared_arrays SHARED_WORDS).

## 2026-10-05 p47-task-list-literal (P47 stage 1)
- `[a, b]` of task variables gave the last result only: inside `[…]` an unbracketed sequence ending in a value (the
  checked await) is a computed element, not a statement (analyzer is_statement).

## 2026-10-05 p47-job-lists (P47 stages 2-4)
- `jobs.add(go f(i))` keeps tasks unawaited; reads of the list await every job, `jobs#i` one, `await all xs` every
  one (notes/threads.md "Job lists"); three one-second jobs take ~1 s.

## 2026-10-05 zero-warnings-2
- Redo of claude/zero-warnings-0h6ky3 on current main (its 13 commits conflicted with two days of changes; its CI half
  was already on main): crate-level allows gone, dead code deleted as that branch chose, native-only items cfg-gated;
  all six CI warning/clippy commands pass locally. The old remote is renamed archive/zero-warnings-0h6ky3.

## 2026-10-06 fix-text-words
- g-0muE: `strip` is trim's synonym (Python), `s.trim` / `s.strip` need no parentheses (library_words SYNONYMS).
- print-empty: `print()` writes an empty line and is worth "", as `print ""` (list_emitter emit_empty_print, analyzer Text).

## 2026-10-06 fix-pipe
- url-ends-at-space: `fetch url | strip` read as `fetch (url or strip)`. Now `x | f` is f(x) when f names a function
  (D6; pipes.rs, the parser marks the word after a single `|`), a braceless call pipes its result
  (`square 2 | root`), `|` between values stays or (its `or` hint only then).

## 2026-10-06 fix-empty-operands
- print-empty-nodes: `(#name as string)` serialized as `(ø#name as string)`; Node::serialize writes a prefix operator's
  missing left operand (#, -, +, not, √, if, while) and a suffix operator's (x++) as nothing, `x = ø` stays.

## 2026-10-06 fix-todo-comments
- warp_parser "todo edge case: leading plus": `+5`, `+x`, `3 + +2`, `[+1 -2]` gave "Unexpected character '+'"; a `+`
  glued to its operand is now the unary plus, the operand itself (a spaced `+` stays the operator word).

## 2026-10-06 lowered-error-text
- `n=3; cube 1..n` said "undefined: cube in `cube (range_value·range=ø; …)`": diagnostic::written_text quotes the source
  from the node's position to the end of its statement when the serialization holds compiler temporaries (·).
  The card's own example `is in of to from` now fails earlier with "undefined variable: from".

## 2026-10-06 build-exe-default
- g-1KS4: `warp build <file>` makes the standalone executable without --exe (still accepted); `warp build --wasm`
  and `warp compile` write the module.

## 2026-10-06 p102-exe-naming (P102, P103)
- `warp hello.warp` runs and leaves the executable `hello` (hello.exe on Windows), rebuilt only when the source is
  newer; a program the stub cannot carry (fetch, read, run) gets a stderr note. build/compile only make it (failure
  exits 1), `--wasm` / `--aot` give the module. The program compiles twice on a fresh run (eval, then the printing
  variant for the executable).

## 2026-10-06 p104-stub-only (P104, P105)
- No executable is ever a copy of warp: without a warp-runtime stub a run notes it, build exits 1; warp's main no longer
  looks for a carried program. `warp run <file>` runs without leaving an executable. Tests build the stub once per run.

## 2026-10-06 panics-remaining
- extensions/numbers.rs: the four `unsupported types` panics and `unimplemented!` go (mixed: complex, exact real, IEEE);
  wisp_parser had no reachable panics (its 13 are test assertions) but repaired malformed input silently: now errors.

## 2026-10-06 run-fast (P105 follow-up)
- `warp run` has no machine-code path; fresh 73 ms is front end + emitter (65 ms), JIT ~8 ms; OptLevel::None no gain,
  so no code change; probes/aot/run_speed.sh and notes/aot.md "Run speed".

## 2026-10-06 fix-diagnostics (user issues #11 #12 #13 #17, g-1pvQ)
- No hints for 'x'/"x", let/:=, str(x)/x as string, a number joining a text: the default Style leaves those axes Any,
  Style::canonical keeps one spelling each (hint-machinery tests run under it). sleep(1) warns for a unit. norm = abs.

## 2026-10-06 guillemet-strings
- «text» ended only at another «: parse_string closes « with ».

## 2026-10-06 hash-index-hint
- g-2WPo: the xs#n hint only for a literal or a name as index (normalize is_simple_index).

## 2026-10-06 flaky-browser
- every_sample_runs_without_a_compiler_error timed out when the task pool was empty (Workers not loaded yet, or used up
  by stopped tasks): host.js then runs a task inline, and threads.warp's `go spin(10^12)`; `stop endless` cannot stop
  an inline task. Reproduced with prepareTaskPool(0) (120 s timeout); fixed by taskPoolReady before each run and a
  replacement Worker for a stopped one. Alone the test takes 3.4 s in the browser.

## 2026-10-09 print-quantity (P231)
- print rounds a quantity fraction to two decimals (6.17km, PRINTED_AMOUNT_TEXT in static_units.rs); str, joins and the result keep (37/6)km.
## 2026-10-09 print-km
- `"a: " + x as km` parsed `("a: " + x) as km`: a conversion after a text join converts the last operand (units.rs with_converted_last_operand).
## 2026-10-09 text-join
- a longer text join with several quantities: static_units takes any sum with a text in it as a join (units::joins_text, shared with print-km).
