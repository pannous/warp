# Test triage 2026-10-28 (decision night-releases-and-test-triage)

The failing lines of test/results.txt (aa60d3ddc3, 4149 ✗), sorted into three groups:
- **gap**: the test is right and warp should do it. These are grouped into cards below and not edited.
- **wrong**: the expectation is wrong. These are listed for the user to approve and not edited.
- **esoteric**: a feature we don't carry over. These are commented out with
  `# triage-held 2026-10-28: <reason> :: <the line>`; an is! test's further lines start with `# :: `.
  To restore one, remove everything up to and including ` :: `, or `# :: ` on a continuation line.

## Wrong expectations (approve to correct)

| file:line | expected now | proposed | why |
|---|---|---|---|
| test/welcoming/test_c_style.warp:26 | "class float" | "class Real" | π is the exact Real (decision exact-constants) |
| test/numbers/test_rational_type.pairs.warp:20, 21 | "real" | "class Real" | type(π) is the class Real, as above |
| test/numbers/test_float_exact_context.pairs.warp:44 | "real" | "class float" | sqrt of a float is a float; √2 is the exact one |
| test/numbers/test_implicit_libm.pairs.warp:15 | 0.8414709848078965 | 42 | `def sin(x) := 42` is the program's own sin, which wins over libm (shadowing per arity) |
| test/text/test_char_parameter_type.pairs.warp:4, 5, 6 | "codepoint" | "class text" | decisions: an untyped parameter given a one-character text takes it as Text (5 even declares t:text) |
| test/text/test_text_type_words.pairs.warp:12, 13 | π | 3.141592653589793 | `π as float` is the float, not the exact π |
| test/lists/test_min_max.pairs.warp:20 | 2 | 6 | `min(a,b):=a*b` is the program's own min, which wins over the prelude's |
| test/lists/test_lists.pairs.warp:61, 68, 72, 81 | [1, 2, 3] / ['a', 'b', 'c'] | 3 / 'c' | `;` separates statements: `(1;2;3)` and `{a;b;c}` give their last, as `a;b;c` gives 'c' (line 73) |
| test/types/test_one_dispatch.pairs.warp:53, 60, 63 | 'A' / yes / 3 | "U" / no / 7 | the program's own `upper`, `add`, `sum` win over the prelude's, as `min` in lists |
| test/types/test_dispatch.pairs.warp:7 | yes | no | `def sort(x):=0` is the program's own sort, as above |
| test/types/test_time_type_names.pairs.warp:4, 5 | "instant" | "class Instant" | type(…) shows a class, as type(π) is "class Real" |
| `tests/fixtures/…` in 18 files, 70 failing lines (test/modules/test_wasm_modules, test_include, test_import_compiled, test_module_signals; test/control/test_reflection_words; test/ffi/test_component_short_use; test/wasm/test_wasm.warp) | `include tests/fixtures/counter` | `include test/fixtures/counter` | the folder is test/ now; with the new path `include "test/fixtures/counter.warp"` works, unquoted paths stay a gap |
| test/parser/test_newline_precedence.warp:18 | 56 | 55 | `fib := it < 2 ? it : …` starts 0, 1: fib(10) is 55 |
| test/ffi/test_foreign_lines.pairs.warp:4 | π | 3.141592653589793 | atan2 is a float function; the exact π comes only from π |
| test/lists/test_list_literal_sign.pairs.warp:12, 14, 15 | [4, 2] / [4] / 4 | [5 -1 2] / … / 5 then -1 | glued-negative: a minus spaced before and glued after starts a new item (`[a -1]` = [a, -1]) |
| test/lists/test_signed_operand_list.pairs.warp:14; test/operators/test_negated_call_operand.pairs.warp:15 | 4 | 5 then -1 | glued-negative: `x -1` is x then -1 |
| test/operators/test_minus_after_callable.pairs.warp:19, 21, 22 | 3 / 2 / 4 | x then -2 … | glued-negative, as above (`a - 1`, `a-1` stay subtraction) |
| test/operators/test_minus_after_callable.pairs.warp:11 | -3 | -2 | call-takes-right: `f -2 + 1` = f(-2 + 1) |

## Held (esoteric)

| file:line | reason | delete? |
|---|---|---|
| test/welcoming/test_footguns.pairs.warp:383 | `fast` floats that reassociate math | delete? |
| test/welcoming/test_footguns.pairs.warp:395 | C float32 suffix `.1f` | delete? |
| test/welcoming/test_footguns.pairs.warp:403, 404, 425 | symbolic surds (√2·√2 exactly 2) | |
| test/welcoming/test_footguns.pairs.warp:414 | symbolic trig, sin(π/6) exactly 1/2 | |
| test/welcoming/test_footguns.pairs.warp:428 | complex exponentials (Euler's identity) | |
| test/welcoming/test_footguns.pairs.warp:442, 445, 446 | function equality by algebra (undecidable in general) | delete? |
| test/welcoming/test_footguns.warp:44 | function equality by boolean algebra | delete? |
| test/welcoming/test_footguns.pairs.warp:262, 263, 266, 267, 268 | tzdata version stamps on times (267 and 268 contradict each other) | delete? |
| test/welcoming/test_footguns.warp:9 | `law` property declarations | |
| test/welcoming/test_footguns.warp:33, 37 | effect annotations `! Pure`, `! Div` | |
| test/numbers/test_log_glyphs.pairs.warp:4-7, 10-12 | log glyphs ⌞ ⌟ (P226), hard to read beside log(x, b) | delete? |
| test/numbers/test_law.pairs.warp:4, test_law.warp:4 | `law` property declarations | |
| test/numbers/test_unit_definitions.pairs.warp:20 | a calendar era as a unit (`2 AD`) | delete? |
| test/numbers/test_unit_prefix_lines.pairs.warp:16, 17 | `name` and `prefix` as declaration words for plain functions | delete? |
| test/text/test_interpolation.pairs.warp:35 | shell-style `$0` as a function's parameter | delete? |
| test/text/test_text_type_words.pairs.warp:9 | symbolic surds, `str(√2)` as "√2" | |
| test/lists/test_element_wise_scalar.pairs.warp:5, 7, 10 | MATLAB anonymous functions `@(x)` | delete? |
| test/lists/test_where_filter.pairs.warp:13 | Swift closure arguments `$0 + $1` | delete? |
| test/operators/test_add_subtract_words.pairs.warp:14, 15, 16 | events named by operator words (`on subtract`, `emit plus`) | delete? |
| test/operators/test_approximately.pairs.warp:19, 20, 21 | truthiness under ≈ (`yes ≈ 2`, `no ≈ ""`) | delete? |
| test/operators/test_operators.pairs.warp:4, 5, 6 | endless loops (`while True : 2`) expected to give 0; each times out after 30 s | delete |

| test/control/test_cell_truth.pairs.warp:4, 5, 6, 9, 10 | internal cell_new/cell_get API, not a language form | delete? |
| test/control/test_text_patterns.pairs.warp:25-31 | internal text_holes helper; text patterns are tested by the match lines | delete? |
| test/control/test_value_methods_are_functions.pairs.warp:5 | methods picked by position (fs#4), an order nobody promises | delete? |
| test/types/test_kind_marks.pairs.warp:4-9, 12-14, 17-20, 23, 24 | kind_number/kind_info/kind_mark expose the internal kind bits, a layout test | delete? |
| test/types/test_real_text.pairs.warp:4, 5, 7, 10 | symbolic surds, √2 printed as "√2" | |
| test/wasm/test_wasm.pairs.warp:252-255 | postfix (RPN) operators, `true true and` | delete? |
| samples/text/word_lengths.warp:5 | `length all xs`, a keyword to broadcast; `xs.map length` is the form (decision sample-word-functions) | delete? |
| test/wasm/test_wasm.pairs.warp:307 | a second block as else without `else`, `if(2){3}{4}` | delete? |

## Gap groups (cards)

### welcoming (155 gaps)
Silently wrong, which matters most:
- `for char in "abc" { n += 1 }` gives 0 (test_golf_words:12, 13).
- A big integer's `rem` gives 0 (test_footguns:362).
- `x.upper!` leaves x unchanged (test_discarded_pure_warning:5, 6, 11).
- `"a" + warning("careful") + "b"` gives "aøb" (test_warning_mode:4).

Groups:
- Dates, times and durations as values: `2024-02-29.month`, `1 hour == 60 minutes`, zoned times. About 25 lines, error "a data".
- A round group `(1 2)` as a list value, mutable and aliased. 13 lines, error "the group (…".
- Assigning to a text element, `x#1 = 'é'`. 8 lines.
- English operator words: plus, minus, equals, greater/less than, at least/most, through. 9 lines.
- Text views: `as bytes`, `as codepoints`, `as graphemes`, `byte in t`, "number of bytes in". 9 lines.
- `xs.insert(v, at: i)`. 7 lines.
- Bool arithmetic, `true + true`, `(1<2) + 1`, `v.has(1) + 1`. 5 lines.
- `round_half_even` and `round_half_up`. 6 lines.
- A method without parentheses, `xs.product`, `s.capitalize`. 6 lines.
- `sleep` with a spaced unit, `sleep(16.7 ms)` or `sleep 5 ms`. 5 lines here, many more in control.
- Float ∞ and NaN, `1.0/0.0`. 3 lines.
- `range(n)`. 3 lines.
- `if x=2` read as a comparison. 3 lines.
- `++i` and `--i`. 3 lines.
- Pairs one at a time: C-style typed declarations, `def … end`, `x <- 3`, `let mut`, `next`, `xs count`,
  shadowing pi and count, sh and sql templates, `fetch`, function equality by definition (f==g alpha-equal).

### numbers (263 gaps)
Silently wrong:
- `2^62 | 2^61` gives 2^62 (test_big_int_bits:18).
- `1 << 100 == 2^100` gives no (test_shift_operators:18).
- `ε*ω` gives 2876127 (test_hyperreals).
- `use units; m = 5; str(3m)` gives "15" (test_units_arithmetic:4, test_unit_beside_variable:4, units_after_numbers:4, 7):
  a meaning question for warp-interviewer.
- The harmonic sum `s += 1/k` floors to -5 (test_unbounded_int:67).
- `sqrt(2).round(2)` ignores its argument (test_method_on_operator_call:4).

Groups:
- Standard unit words without `use` (decision implicit-standard-words, e7d456f44e: `ms`, `km`, `minute` work without
  `use units`; covered by samples-implicit-use). 83 lines: test/numbers/ test_decimal_units:4; test_dimension_types:4-7, 19-23; test_dynamic_units:4, 5, 8, 9, 12; test_litre_hectare:4; test_math:42; test_mixed_quantity_arguments:4, 5; test_mixed_units.warp:4, 6; test_pixel_unit:4, 5, 9, 13 and .warp:6, 8, 14; test_quantity_cast:4; test_run_time_power:6; test_time_of_day:4, 6, 9, 13; test_unit_beside_variable:10; test_unit_definitions:4-9, 11, 14, 15, 24, 26, 27, 30 and .warp:4, 7, 9, 11, 15; test_unit_functions_as_values.warp:4, 9; test_unit_literals:4-6; test_unit_prefix_lines.warp:6; test_unit_word_keys:13, 14; test_units_after_numbers.warp:30; test_units_compare:4-14; test_units_type_holes:4-7.
- Unbounded ints past 64 bits, the D16 rule: they trap with cast failure or unreachable (big_int_bits, big_ratio,
  int_division, unbounded_int, ratio_gcd_warp, shift_operators). About 45 lines.
- A spaced unit after an assignment, `x = 2 km`: "a value after an assignment". 20 lines.
- The `real`, `exact` and `rational` types as parameters and variables: "a parameter of kind rational". 20 lines.
- `type([…])` of a list: "the type of a list". 15 lines.
- Repeating decimals `0.3̅` and `as decimal` (decision repeating-decimals). 15 lines.
- `fast` functions and scopes (decision fast-scope); `use fast_math` doesn't reach pi/tau and fails "checked multiply". 30 lines.
- Zero-filled arrays, `640000*int` and `x : 1000 int`. 10 lines.
- Unit definitions: `unit furlong := …`, affine `unit rankine as kelvin`, `x in minutes`, `kilometers`. 15 lines.
- `nearest(x, rational)`, the best rational approximation. 8 lines.
- ∞ needs `use reals`, though π doesn't; NaN and ∞ from float division by zero. 9 lines.
- Methods without parentheses, `x.floor`, `x.abs`. 6 lines.
- Fixed-width C types (`unsigned short`, `long long`, `int32` variables). 5 lines.
- One-offs: `log(x, base)`, `ln`, `∜`, `int("1e3")`, `else` as or-else (`false else 3`), `{a:1}(a)`, `a-b` names,
  `↑`/`↓` reaching log, "size of … in bytes", number words (`one plus two`), long decimal literals kept exact.

### text (100 gaps)
Silently wrong:
- `add "c" to x` and `x.add("c")` on a text give "ab['c']" (test_add_to_text, 4 lines).
- `try ord("ab") else 0` gives 97 (test_character_comparison:35).
- A class's `text(p:P)` isn't used by `s + p`, `x as string` or `String(x)` (test_text_plus_printable:16, test_text_type_words.warp:4, 5).
- `for byte in "äb" { byte }` gives "b" (test_text_unit_lists:7).

Groups:
- `count x in y`, `.count(x)` and `count(xs, x)`: "cannot access length code of bool". 18 lines.
- Assigning to a text element `k#1='a'`. 14 lines, the same group as in welcoming.
- sh and sql templates as lists. 8 lines.
- `byte_slice`. 6 lines.
- Selector prepositions, `length of words in t`. 6 lines.
- Backtick templates. 6 lines.
- `split` with no argument. 4 lines.
- `.string` of a list, `type([…])`, `use js` in a native run, `print()` with no argument, `join xs sep`,
  `Cursor.take_while(is_letter)`, `2nd char in`, `'world'#-1`, Python slices `t[a : b]`, text * a float count.

### lists (243 gaps)
Silently wrong:
- `[a -1]` gives [4] and `[1 -1]` gives [0]. The tests expect a sign glued to its number to start a new item (test_list_literal_sign, test_signed_operand_list, 10 lines).
  Expectations such as `[-9 -8]` are read the same way, so element_wise_vectors.warp:28 fails even though it gets [-9 -8].
- `p.@source = "gps"; #p` counts the meta field (test_map_starts:9).

Groups:
- A round group `(1 2 3)` as a list. 25 lines.
- Element-wise `.+ .* .- ./` and scalar broadcasting; `dot`; math words over lists. 45 lines.
- `linear` arrays. 17 lines.
- Swizzles `.x .xy .zyx` (decided). 16 lines.
- `where` filters and comprehensions. 14 lines.
- Lazy ranges counted without materializing (time out). 10 lines.
- `pop(i)`, `clear()`, `delete xs[i]` and their aliasing. 15 lines.
- Unpacking: `a, *rest =` and `f()` returning several values. 11 lines.
- `x : 100 int` and `int[100]` declarations. 10 lines.
- `a=1 2 3`, a bare list after `=`. 6 lines.
- `===` identity. 5 lines.
- `xs.insert(x, at: i)`. 4 lines.
- A typed list variable `xs: [P] = …`: "an assignment to `… : …`". 4 lines.
- `pixels size` and `pixels count` as postfix words. 5 lines.
- `sort_by`, `h.0` numbered fields, `map<int,int>`, `dict.fromkeys`, `tuple`, `values` of a pair.

### operators (159 gaps)
Silently wrong:
- `n = 2; m = 4; n | m` gives 2 (test_bit_operators_on_ints:21), and `-4|abs` gives -4. `|` is both bit-or and the
  pipe: settle the rule.
- `x < 2 then 1 else 2` with x = 3 gives 1 (test_then_pipe:11).
- `x and= no` and `x or= yes` leave x unchanged (test_operator_spellings:33, 34).
- The `!` bang leaves its variable unchanged: `sort xs!`, `reverse xs!`, `twice x!`, `x.upper!` (test_mutating_bang, test_print_bang).
- `square numbers |> filter(…) |> sum` gives 0 (test_pipeline.warp:12).

Groups:
- `≈` and `~` on numbers, texts, lists, maps and classes, with a tolerance (alike, approximately, rough_similarity). 45 lines.
- English operator words (`plus`, `add`, `divided by`, `greater than`, `the same as`, `circa`). 30 lines.
- The `|` pipe and `then` pipe over function names, with `square x := …` shadowed by `use math`. 30 lines.
- A negated or minus-applied call, `-floor 2.5` or `int8 -3`: "unknown name floor". 12 lines.
- Postfix words `-7 abs` and `16 sqrt` trap with a null reference; `10%` as a percent. 12 lines.
- Chained comparisons, `1<2<3` and `1<2==2`. 8 lines.
- `root`, `function abs` as a value, `xs.map(abs)`, `switch` with list patterns, `"3" == 3`, `like`, `import regex`.

### control (891 lines; results.txt predates the event-handlers-unknown merge 59583676fb, so the event count is high)
Silently wrong:
- `match 2 { 1, 2 => "small"; _ => "big" }` gives "big"; `match 1 { 1, 2 => "small" }` gives 1 (test_match_alternatives:10-12).
- `x = data (n * 1); x is data($a * 1)` gives no (test_code_holes.warp:44); code-statement patterns on `parse(…)` give "no" (test_code_statement_patterns).
- `string(data point{x:1})` gives "point:{x:1}", `data a:1` prints without braces (test_data_quoting:12-18).
- `for char in text`, `for byte in t`, `for line in` iterate wrongly or not at all (test_for_forms:24, 25, .warp:7; test_for_unit_words:5, 12, 13), the welcoming `for char` group.
- Class-filter loops `for friend in xs` count the wrong elements (test_filter_loops.warp:4, 5).
- `"a" + warn("careful") + "b"` gives "aøb" (test_error_words:9), as warning() in welcoming.

Groups:
- Events and signals: `on`/`once`/`whenever`/`emit`, `on set`/`on change` with value/old, listeners of, `before`/`after`, events across tasks
  (reactive_ports, variable_signals, event_footguns, event_handler_lists, page_events, signal_*). About 250 lines before the event merge.
- Reflection: `.params .signature .doc .dump`, `methods/fields/properties/attributes/dir`, `help`, `type(f)` as a signature,
  `use reflection` positions (reflection_words, reflection_synonyms, function_objects, library_signatures, doc_strings, help_word). About 125 lines.
- Tasks: `go`, `await (all)`, `shared`/`atomic` values and arrays, `pause/resume/stop job`, `@parallel`, channels, `sleep(500 ms)`. About 95 lines.
- Match and switch: alternatives, tuples, list and class patterns with `..`, text patterns `"v" + n: digits`, ranges. About 85 lines.
- Stores: `storage`, `local`, `session`, `database`, `stored x default …`, values across runs. About 50 lines.
- Quasi-quotes and code patterns: `data($a + 1)` splices, `$rest...`, `replace data(…) with …`, tree find/replace. About 45 lines.
- Run-time blocks: `interpret`, `x!` charging a quoted block, `x : a+b` uncharged blocks. About 30 lines.
- `try … else` over traps: list index, division, named traps, stack overflow, host failure, `break`/`continue` inside try. 28 lines.
- One-offs: `on every … at 9:00` timers, `undo x`/`redo x`, `for (even number) in`, `[x for x in xs where …]`, Rust `let mut`/`1..=5`/`loop`,
  `int? i`, `def f(x) as int or error`, pattern schemas `{port: int} is …`, soft `test`/`check` blocks, `exit`, the `is!` word in programs.

Questions for 10:00 (forms nobody has decided are wanted, per tests-serve-the-language):
- Multi-word event names, `on stop the machine { … }; emit stop the machine` (test_event_signals.warp:4, test_named_broadcast:5, 8, test_reactive_ports.warp:53).
- Python `try: … except ZeroDivisionError:` blocks (test_try_catch_except.warp:8).
- `undo x` / `redo x` history for every variable (test_undo_history, 10 lines).
- `on error of f { 0 }`, a handler attached to a function's failures (test_on_error_of_function, 7 lines).
- Fixed mouse values in tests, `mouse_x * 100 + mouse_y` → 1207 (test_system_values:26, 27): is there a stubbed pointer, or should these go?

### types (733 lines)
Silently wrong:
- `Person{name:"A" hobbies:[1 2]}.name` gives "Person": the tag name shadows the field (test_tagged_objects:4).
- `F#4` on a class with `index(o)` gives 503 (test_class_index_operator:4): the pending user question class-index-test, not a bug yet.
- `int 3.7 == 3` gives 0 (test_nested_type_calls:15).
- `class Bag{items}; def sum(b:Bag):=7; sum([1,2])` gives 7: a typed overload hides the prelude's sum for lists (test_dispatch:15).
- A class's `text()`, `toString()`, `to_s()` and `equals(o)` aren't used by `as text`, printing in lists, `==` or `in`
  (test_class_witnesses:5, 6, 10, 12; test_printable_items; test_instance_final_text; test_trait_equals_membership).
- `use json; to_json(Point(1, 2))` wraps each object in its class name, `{"Point":{"x":1,"y":2}}` (test_class_json, 16 lines).
- `ps.copy()` shares the instances: `qs#1.x = 9` changes p (test_copies.warp:4).
- `p.f = 5` on a class with an external `f(p: P, v)` setter, and property `set` blocks, do nothing (test_class_properties, test_one_dispatch:19).
- `xs: floats = [1/3, 2]; xs[0] is float` gives no: declared floats keep the exact ratio (test_number_converts:4, 7, 9).
- `names: texts = ["hi"]; ys.add(1)` adds an int to a texts list (test_declared_list_marks.warp:4).

Groups:
- Dates, times, durations and zones as values: "a data". About 100 lines (duration_comparison, time_values_at_run_time,
  time_zones_at_run_time, time_variables, time_now, date_arithmetic_at_run_time, partial_time_comparisons): card triage-time-values.
- Type values and aliases: `class int == int`, `type(ø) == empty`, `integer == int`, `type(now) == instant`, `type([…])`. About 50 lines
  (empty_type_aliases, bare_class, type_tests, types, type_alias_fields).
- Declarations without a value and sized arrays: `x : int`, `x: [int] = …`, `int[3]`, `typed arrays` ("an assignment to `… : …`",
  "unknown name x"). About 60 lines (array_types, typed_arrays, declaration_without_value, sized_array_values, collection_declarations).
- Traits and interfaces: `trait`, `interface`, default methods, conformance, `Comparable`. About 30 lines.
- Sum types, enums and destructuring in match: "no case for value". About 25 lines (class_destructuring, sum_types, tagged_unions, enum_cases, variant_payloads).
- `type(f)` as a signature, "function: (x) -> int", instead of "class function". About 15 lines here, more in control.
- Code as a type: `code` values and methods on them, "a function value in .". 13 lines (test_code_type).
- Checked results `def f() -> int or error`, union returns. About 20 lines.
- `===` identity on ints and lists, `same`. About 15 lines (same_identity, passed_list_identity).
- Classes: mixins, extension methods, `this`, class forms and cases from the Rust port, generic classes, recursive fields. About 60 lines.
- `flags`, `hex(…)`, `double(…)`, `x :: int`, `as` on int, bool + int. About 35 lines.

Questions for 10:00:
- `s = "a"; s++` → "a1" (test_chained_assignments:10): increment a text by appending 1? Today it gives "b".
- `x is number 9` → yes, "is a number and equals 9" (test_is_declaration:7, 9).
- `b = byte 65; text(b)` → "0x41" (test_type_calls:11): bytes print in hex?
- `to_json` of a class: plain fields `{"x":1,"y":2}` as the tests expect, or tagged with the class name as now?

### functions (618 lines)
Silently wrong:
- A function value applied without parentheses isn't called: `sq = x=>x*x; sq 3` gives the list `x => x*x 3`; the same
  with `g = function add; g 1 2`, `g := &doubled` (test_lambdas:4, 6, 11; test_function_values:12; test_braceless_closure_call:5, 6; test_alias_definition:15).
- `random` with an argument and no parentheses gives a list `[float, arg]`: `random 1..2`, `random [7,7]`, `random n`, `random int`
  (test_random_dispatch, test_bare_host_words:25-36, about 15 lines).
- `map square xs`, `map floor [1]`, `map (x -> x*2) [1,2,3]` give a list (test_function_words_consume:8, test_map_library_word_first:12, test_ocaml_fun:4).
- `(() => 7)()` gives the lambda (test_anonymous_function_forms:17); `f(x=1):=x*2; f` gives f, not 2 (test_bare_function_name:7).
- `global n` inside a lambda doesn't write the global: n stays 0 (test_lambda_global:4, 5); a closure counter returns 1 each time (test_nonlocal_lambdas.warp:4).
- A Ruby block `[1,2,3].sum { |x| x*2 }` is ignored: 6 (test_round8_call_forms:4, 5).
- `greet({name}) := …` doesn't destructure: "hi {name:"Ann"}" (test_parameter_map_patterns.warp:4).
- `"äb".size in bytes` gives no (test_member_phrases:13).

Groups:
- Preposition calls and multiple return values trap with cast failure (preposition_calls, multi_value, words_in_phrases). About 45 lines.
- `nonlocal` and late-bound globals (late_binding, nonlocal_*). 22 lines.
- Foreign definition forms: Python/Ruby `def … end`, `lambda`, `fn`, `function`, Swift closures, Rust `&` references, OCaml `fun`
  (python_ruby_definitions, foreign_spellings, rust_references, swift_closures, anonymous_function_forms). About 75 lines.
- Implicit `it` in blocks and nested `it` ("unknown name it"). About 25 lines (nested_it, it_arguments, returned_blocks).
- Suffix words `3 squared`, `x.squared`, and math words without `use math` ("`use math` defines it"). About 30 lines.
- Named and labelled arguments, defaults and optional parameters. About 30 lines.
- Variadic `*xs`, tuple returns and tuple function values ("index out of range"). About 30 lines.
- Callable parameters `f: (float) -> float`, function values in fields and lists. About 30 lines.
- Element-wise broadcasting of functions over lists: card triage-element-wise. 7 lines.
- `render … as …`, `sorted_node`, `random` with 2 arguments, `x(…)` calls of variables, `.` on int.

Questions for 10:00:
- Which wins when a program defines a word the prelude has: the program's own (as test_implicit_libm, test_min_max, test_one_dispatch:53-63 and
  test_dispatch:7 would then need new expectations) or the builtin (as test_builtin_clash:5, 14-17 expects: `count(x) := 7; count([1,2])` → 2)?
  test_arity_overloads.warp:4 (`def log(x) { log(x, 10) }; def log(x, base) {x/base}; log(50)` → 3.912) fits neither. The rows in
  "Wrong expectations" for min, sin, upper/add/sum and sort assume the program's own wins.
  Related: decision cross-module-overload (overload by argument type, no shadowing) and merge 90b2efe396 (a program's function
  named like a host word shadows it per arity); neither says what an untyped `count(x) := 7` does to the prelude's count.
- `p's name` as a field read (test_property_access:14).

### modules (229 lines)
Silently wrong:
- A program's helper names leak into the prelude: `round_to(x, d) := 99; round(2.555, 2)` gives 99, and `first_items` likewise
  (test_prelude_shadowing:4, 6). This is a hygiene bug whichever way the shadowing question goes.
- `use draw; global hue = 10; hsv(120, 1, 1); hue` gives 2: a module writes the program's global (test_use_modules:16).
- `use collections; OrderedMap()` lists an internal key "view·opaque" in keys(m) (test_std_words.warp:4); LinkedHashMap counts 2 keys for 1 (test_std_collections:35).
- `use json; parse("{\"a\": 1}") is map` gives no (test_json_parse_form.warp:4, 7).
- `include …fourty_two` gives ø; an imported wasm `letters("hello")` gives the codepoints instead of 5 (test_wasm_modules:11, 21).

Groups:
- Stale `tests/fixtures` paths: 70 lines across folders (Wrong expectations above); after the fix, unquoted include/import paths are the gap.
- `dir(…)` and `function count` of builtin members. About 25 lines (dir_builtin_members, dir_any_value, dir, module_function_dir).
- Packages and versions: `use uniscript >= 1.0`, `use colors version 1.0.0`, the registry. About 25 lines (versions, package_registry, packages, selective_package).
- std collections: LinkedHashMap, OrderedMap, Deque, `remove(…)`. About 25 lines.
- `require`/`import`/`use` of a single word like `square`; `from … import …`. About 15 lines.
- The rewrite engine: `rewrite scaled $x => …` and `·` as an operator. 8 lines.
- std: regex, matrix, math text, exec, crc32/XXH hashes, json5. About 20 lines.

Questions for 10:00:
- `first(x) := 7; first([1])` → 1 (test_std_prelude:21) is the builtin-wins side of the shadowing question in functions.

### parser, ffi, wasm, programs, node, scope, web, sweeps (579 lines)
Silently wrong:
- Unary plus is a symbol: `+5` gives `'+' 5`, `[+1 -2]` gives ['+' -1] (test/parser/test_unary_plus, 6 lines).
- `¬ 1`, `¬ false` with a space give "no 1" (test/wasm/test_wasm:268, 269, 283; test/parser/test_surface_syntax:17).
- A trailing backslash doesn't continue the line: `1 + \` newline `2` gives 2 (test/parser/test_parser_sugar.warp:16, 20).
- Juxtaposed calls in `and`: `sq 2 == 4 and sq 3 == 9` gives 0, `f 3 and f 5` gives 20 (test_and_after_comparison:5, test_statement_phrases:4).
- `fib := it<2 then 1 else …` gives 1 and `it<2 and it or …` gives -11 (test/wasm/test_wasm:31, 32, 312, 313).
- `for i in (1..3)` sums to 3 (test/wasm/test_wasm:477, .warp:64).
- `π*1000000.` stays exact, 1000000π, though `1000000.` is a float (test/wasm/test_wasm:379).
- Meta fields count as fields: `p.@s = 30` changes `for k,v in p` and `#p.keys` (test/parser/test_meta_attributes:23, 27), as in lists.
- `use c; strstr("haystack", "st")` gives 1 instead of "stack" (test/ffi/test_libc_results.warp:26).
- `prefix operator ∂ := sqrt; ∂16` gives '∂' 16 (test/parser/test_operator_aliases.warp:6).

Groups:
- `use sound` words without `use` (samples, note values, sound pipes, web sounds): about 60 lines, covered by implicit-standard-words / samples-implicit-use.
- test/wasm/test_wasm is the old Rust catch-all: 110 lines over xor, puti, `global x = …`, log glyph calls, `⅓x`; it should be split into topic folders.
- Rust surface forms: paths `a::b`, macros `println!`, items, `mut`, references (rust_paths, rust_macros, rust_items, references). About 40 lines.
- `±` and surface operators (test_surface_syntax). About 27 lines.
- Meta attributes `@source:` in maps: "not a function". 10 lines.
- Foreign JS/Python/WebIDL/cargo crates (`Math`, `stringify`, `Regex`). About 60 lines in ffi.
- Node: `interpret`, quoted code whose position or comment is read, `.op` of a node. About 25 lines.
- Scope: `global x = …` declarations, `result`, `export`, `a-b` names. About 30 lines.
- `√3^2` gives 2.9999999999999996: the card root-power-warning (√ and ^ without grouping).

Questions for 10:00:
- `use sound; note_value(E4)` expects the text 'E4' but gets the note E4 (test/programs/test_note_values, 11 lines): should a note print as text, or should the expectations become E4?
- `mouse_wheel` → -2 (test/programs/test_mouse_wheel:4), like `mouse_x` in control: a stubbed pointer or remove?
