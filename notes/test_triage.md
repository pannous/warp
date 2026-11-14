# Test triage (decision night-releases-and-test-triage)

The failing lines of test/results.txt (aa60d3ddc3, 4149 ✗), sorted into three groups:
- **gap**: the test is right and warp should do it. These are grouped into cards below and not edited.
- **wrong**: the expectation is wrong. These are listed for the user to approve and not edited.
- **esoteric**: a feature we don't carry over. These are commented out with
  `# triage-held 2026-10-28: <reason> :: <the line>`; an is! test's further lines start with `# :: `.
  To restore one, remove everything up to and including ` :: `, or `# :: ` on a continuation line.

## Wrong expectations (approve to correct)

| file:line | expected now | proposed | why |
|---|---|---|---|
| test/numbers/test_float_exact_context.pairs.warp:44 | "real" | "class float" | sqrt of a float is a float; √2 is the exact one |
| test/numbers/test_implicit_libm.pairs.warp:15 | 0.8414709848078965 | 42 | `def sin(x) := 42` is the program's own sin, which wins over libm (shadowing per arity) |
| test/text/test_char_parameter_type.pairs.warp:4, 5, 6 | "codepoint" | "class text" | decisions: an untyped parameter given a one-character text takes it as Text (5 even declares t:text) |
| test/text/test_text_type_words.pairs.warp:12, 13 | π | 3.141592653589793 | `π as float` is the float, not the exact π |
| test/lists/test_min_max.pairs.warp:20 | 2 | 6 | `min(a,b):=a*b` is the program's own min, which wins over the prelude's |
| test/lists/test_lists.pairs.warp:61, 68, 72, 81 | [1, 2, 3] / ['a', 'b', 'c'] | 3 / 'c' | `;` separates statements: `(1;2;3)` and `{a;b;c}` give their last, as `a;b;c` gives 'c' (line 73) |
| test/types/test_one_dispatch.pairs.warp:53, 60, 63 | 'A' / yes / 3 | "U" / no / 7 | the program's own `upper`, `add`, `sum` win over the prelude's, as `min` in lists |
| test/types/test_dispatch.pairs.warp:7 | yes | no | `def sort(x):=0` is the program's own sort, as above |
| `tests/fixtures/…` in 18 files, 70 failing lines (test/modules/test_wasm_modules, test_include, test_import_compiled, test_module_signals; test/control/test_reflection_words; test/ffi/test_component_short_use; test/wasm/test_wasm.warp) | `include tests/fixtures/counter` | `include test/fixtures/counter` | the folder is test/ now; with the new path `include "test/fixtures/counter.warp"` works, unquoted paths stay a gap |
| test/parser/test_newline_precedence.warp:18 | 56 | 55 | `fib := it < 2 ? it : …` starts 0, 1: fib(10) is 55 |
| test/ffi/test_foreign_lines.pairs.warp:4 | π | 3.141592653589793 | atan2 is a float function; the exact π comes only from π |
| test/lists/test_list_literal_sign.pairs.warp:12, 14, 15 | [4, 2] / [4] / 4 | [5 -1 2] / … / 5 then -1 | glued-negative: a minus spaced before and glued after starts a new item (`[a -1]` = [a, -1]) |
| test/lists/test_signed_operand_list.pairs.warp:14; test/operators/test_negated_call_operand.pairs.warp:15 | 4 | 5 then -1 | glued-negative: `x -1` is x then -1 |
| test/operators/test_minus_after_callable.pairs.warp:19, 21, 22 | 3 / 2 / 4 | x then -2 … | glued-negative, as above (`a - 1`, `a-1` stay subtraction) |
| test/operators/test_minus_after_callable.pairs.warp:11 | -3 | -2 | call-takes-right: `f -2 + 1` = f(-2 + 1) |
| test/control/test_database_tables.warp, test_database_optional.warp, test_database_short_names.warp (all `stored x: [C] in "t"` lines) | rows in the table before the line runs | each line its setup first, `pets.add(Pet("Rex"))` before `count(pets)` | the Rust tests wrote the rows with `eval(…)` lines the conversion dropped (saved/archive/rust-history tests/control/test_database_tables.rs:30); alone each is! reads an empty table |
| test/wasm/test_wasm.pairs.warp:478 | 6 (`for i=1..3;sum+=i`) | 3, or written `for i=1 to 3` for 6 | decision range-dots: `..` is exclusive; :477 and test_wasm.warp:64 were rewritten to `1 to 3` (user-approved), this one is still a gap too (`for i=…` refused) |
| test/numbers/test_time_of_day.pairs.warp:4, 6 | "still today" / 1 (`if now < 24h`, `now >= 0 ms and now < 1 day`) | a clock reading: `if now.hour < 24`, the day in hours: `now < 24 hours` | decision instant-vs-duration: `now < 24h` compares the day of the month, a calendar `1 day` beside a clock duration is refused (stale, user 11-11 via the Interviewer, card spaced-unit-comparison) |
| test/functions/test_shared_captures.pairs.warp:11 | 10 (`n=1; f = () => n * 10; n = 2; f()`) | 20 | decision captured-same-variable: a captured variable is the same variable, so f sees the later n = 2 |

## Held (esoteric)

| file:line | reason | verdict |
|---|---|---|
| test/numbers/test_unit_literals.pairs.warp:4, 5, 6, 9, 10, 11; test/numbers/test_unit_definitions.pairs.warp:22, 35; test/numbers/test_unit_prefix_lines.pairs.warp:13 | spelled unit names after a number: `3 kilometers`, `3010 meters`, `2 kilograms`, `3 meter`, `3 feet in inches` | parked: spelled-units-number (user 2026-11-13: `quantity(3, "meters")` is the way; via warp-interviewer 11-14 the lines stay red, not dropped) |
| test/types/test_time_now.pairs.warp:4-6 | `type(time.now)` 'time of day', `time.now is time`, `time.now.hour == now.hour` (old Rust test, no wiki line) | dropped 2026-11-13 via warp-interviewer: `now.time` (lines 10-11) covers it |
| test/modules/test_wasm_modules.pairs.warp:34, 35 | a C module's out-pointer as the result: `strtol("42 apples", 10)` expected ' apples', `strtol("ff!", 16)` '!'; main gives the C meaning, 42 / 255 | keep for now: the user expects 42 (11-11 via warp-interviewer); the out-pointer form is card strtol-out-pointer (Later) |
| test/text/test_count_in.pairs.warp:4-9; test/text/test_counts_in_warp.pairs.warp:4, 5, 10; test/functions/test_words_in_phrases.pairs.warp:11, 12 | `count x in y`, `count(x in y)`: the count of a membership's yes/no, refused with did-you-mean `y.count(x)` | delete: the refusal with its fix `y.count(x)` is the behaviour |
| test/welcoming/test_footguns.pairs.warp:383 | `fast` floats that reassociate math | keep: wiki Footguns.md:764 promises `fast x=3.3` / `as fast`; gap |
| test/welcoming/test_footguns.pairs.warp:395 | C float32 suffix `.1f` | keep: wiki Footguns.md:772, number.md:28 promise `0.1f`; gap |
| test/welcoming/test_footguns.pairs.warp:403, 404, 425 | symbolic surds (√2·√2 exactly 2) | gap: restore under exact-roots (√2·√2 = 2) |
| test/welcoming/test_footguns.pairs.warp:414 | symbolic trig, sin(π/6) exactly 1/2 | keep held: symbolic trig, no decision yet |
| test/welcoming/test_footguns.pairs.warp:428 | complex exponentials (Euler's identity) | gap: restore now that complex numbers exist (card euler-identity) |
| test/welcoming/test_footguns.pairs.warp:442, 445, 446 | function equality by algebra (undecidable in general) | delete |
| test/welcoming/test_footguns.warp:44 | function equality by boolean algebra | delete |
| test/welcoming/test_footguns.pairs.warp:262, 263, 266, 267, 268 | tzdata version stamps on times (267 and 268 contradict each other) | keep: wiki Footguns.md:845 promises `[_tzdata=…]`; fix the contradicting line, else gap |
| test/welcoming/test_footguns.warp:9 | `law` property declarations | keep: planned (user; wiki laws.md, card verify-law) |
| test/welcoming/test_footguns.warp:33, 37 | effect annotations `! Pure`, `! Div` | rewrite to `@pure`/`@io` under effect-annotations, else delete |
| test/numbers/test_log_glyphs.pairs.warp:4-7, 10-12 | log glyphs ⌞ ⌟ (P226), hard to read beside log(x, b) | keep: wiki operator.md:46 promises ⌟ and ⌞; gap |
| test/numbers/test_law.pairs.warp:4, test_law.warp:4 | `law` property declarations | keep: planned (user; wiki laws.md, card verify-law) |
| test/numbers/test_unit_definitions.pairs.warp:20 | a calendar era as a unit (`2 AD`) | delete |
| test/numbers/test_unit_prefix_lines.pairs.warp:16, 17 | `name` and `prefix` as declaration words for plain functions | delete |
| test/text/test_interpolation.pairs.warp:35 | shell-style `$0` as a function's parameter | delete |
| test/text/test_text_type_words.pairs.warp:9 | symbolic surds, `str(√2)` as "√2" | gap: restore under exact-roots (√2 shows √2) |
| test/lists/test_element_wise_scalar.pairs.warp:5, 7, 10 | MATLAB anonymous functions `@(x)` | delete |
| test/lists/test_where_filter.pairs.warp:13 | Swift closure arguments `$0 + $1` | delete |
| test/operators/test_add_subtract_words.pairs.warp:14, 15, 16 | events named by operator words (`on subtract`, `emit plus`) | delete |
| test/operators/test_approximately.pairs.warp:19, 20, 21 | truthiness under ≈ (`yes ≈ 2`, `no ≈ ""`) | delete |
| test/operators/test_operators.pairs.warp:4, 5, 6 | endless loops (`while True : 2`) expected to give 0; each times out after 30 s | delete |

| test/control/test_cell_truth.pairs.warp:4, 5, 6, 9, 10 | internal cell_new/cell_get API, not a language form | delete: internal API (outcome tests only) |
| test/control/test_text_patterns.pairs.warp:25-31 | internal text_holes helper; text patterns are tested by the match lines | delete: internal helper (outcome tests only) |
| test/control/test_value_methods_are_functions.pairs.warp:5 | methods picked by position (fs#4), an order nobody promises | delete |
| test/types/test_kind_marks.pairs.warp:4-9, 12-14, 17-20, 23, 24 | kind_number/kind_info/kind_mark expose the internal kind bits, a layout test | delete: internal kind bits (outcome tests only) |
| test/types/test_real_text.pairs.warp:4, 5, 7, 10 | symbolic surds, √2 printed as "√2" | gap: restore under exact-roots |
| test/wasm/test_wasm.pairs.warp:252-255 | postfix (RPN) operators, `true true and` | delete |
| test/wasm/test_wasm.pairs.warp:307 | a second block as else without `else`, `if(2){3}{4}` | delete |

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
- Swizzles `.x .xy .zyx`. 16 lines.
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
- Run-time blocks (decision blocks-run-by-call): a block runs by `x()`, a data value by `interpret x`, `x!` only
  unwraps; `x : a+b` uncharged blocks. About 30 lines.
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

## Batch 2 (card apply-verdicts)

Applied: group A deletions without a wiki or board hit (desk-invented -ed suffixes, `pixel number`, `type function a`,
`now ≈ January`, postfix function words, selector phrasings, suffix words, type-word chains and filter loops, bare else;
unknown names as symbols, decision lib-oddities; xs.name broadcast, decision each-field-broadcast); restored the `===`
lines of the identity groups (all green; `!==`, `same`, `same as`, `is the same as` stay held) and the symbolic surds
(√π², √3^2, still red: card sqrt-pi-squared).

Group A forms moved to keep by a hit (the safety rule), for the Interviewer:
- desk-invented count phrasings (`#(t as bytes)`, `number of bytes in t`, `size of t in bytes`): wiki Footguns.md:732,
  string.md:9, and the welcoming gap group "Text views" above
- unit loop heads `for bytes in t`: string.md:38
- loop heads `for (it>2) in xs`, colon loops `each xs: …`: for.md:47, iteration.md:59
- phrase definitions `to greet …`, `to render text as pdf`, `to boop x`: class.md:36, polymorphism.md:57
- `expect 30 10+20`: keyword.md:115
- `map square on xs`: functor.md:9, function-pointer.md:18
- `add(…, overflow: clamp)`: Footguns.md:635, time.md:39
- parent references `ref a`, `$a`: reference.md:15, keyword.md:251
- `x be 3`, `x be number 9`: assignment.md:5, 39 (`x be value` is the definition form)
- dashed keys `a-b:2`: field.md:69 (a declared field `phone-number` is read whole)
- `p's name`, `square of a number = …`: function.md:5
- `listeners of alarm -= 2`: Footguns.md:49-55 promise `remove h from listeners of alarm` (not `-=`)
- `rough_tolerance`: operator.md:41
- `do add 4 to pixel`: ambiguity.md:196 (a Rust-era assert listed there)
- ordinal phrasings `2nd char in t`: card ordinal-index-words
- `class int` values: card type-word-values
- reflection synonyms (methods of, fields, properties, attributes, members): reflection.md:94, cards reflection-words,
  reflection-dot-forms
- unit prefix/name declarations: the user's todo, card prefix-keyword

Groups batch 2 did not name (count, reason, one example, wiki pages holding the example's code verbatim; – for none):

| n | reason | example | wiki |
|---|---|---|---|
| 23 | type name synonyms (unit, nil, empty, string, integer, real) | `'x = ø; x is unit'	true` (test/types/test_empty_type_aliases.pairs.warp:4) | – |
| 13 | old wasp definition by space, `square x := …` | `'square x := x*x; square 2|sqrt'	2` (test/operators/test_pipe_stages.pairs.warp:13) | – |
| 11 | reflection synonyms (properties, fields, attributes, functions of), one word each is enough | `'m = {a:1}; properties(m)'	['a']` (test/control/test_reflection_synonyms.pairs.warp:4) | – |
| 9 | undo/redo word | `'x = 1; x = 2; x = 3; undo x; x'	2` (test/control/test_undo_history.pairs.warp:4) | – |
| 9 | a return type after as / ->, def f(x) as int or text {…} | `'def f(x:int) as int or text { "a" }; f(2)'	'a'` (test/types/test_union_returns.pairs.warp:4) | – |
| 9 | desk-invented count phrasings (char in t, byte in t, t as bytes/graphemes/codepoints, number of bytes in) | `'#(char in "héllo")'	5` (test/welcoming/test_footguns.pairs.warp:187) | – |
| 8 | reflection synonyms (methods of, description, returns, representation, full), one word each is enough | `is! 'use math` (test/control/test_function_objects.warp:7) | – |
| 8 | typed declarations without a value (pixel:int[100], numbers x =, x : 100 * int) | `'pixel:int[100]; pixel#1=5; pixel#1'	5` (test/types/test_array_types.pairs.warp:18) | – |
| 7 | typed declarations without a value (pixel:int[100], x : 100 numbers) | `'pixel:int[100];pixel[1]=15;pixel[1]'	15` (test/lists/test_lists.pairs.warp:19) | – |
| 7 | old wasp definition by space, `add1 x:=…` | `'add1 x:=it+1;add1 3'	4` (test/wasm/test_wasm.pairs.warp:16) | – |
| 6 | conversion as a field, x.string / x.int / x.float | `'x = 1; x.string + "!"'	'1!'` (test/functions/test_conversion_methods.pairs.warp:4) | – |
| 6 | desk-invented phrase definitions (to greet person{…} do) / dashed fields | `'f(p{phone-number:text}) := p.phone-number; f({phone-number:"8"})'	'8'` (test/functions/test_parameter_shapes.pairs.warp:14) | – |
| 6 | desk-invented count phrasings (size of t in bytes, number of bytes in, #bytes in) | `'t="héllo";number of bytes in t'	6` (test/lists/test_size_count.pairs.warp:14) | – |
| 6 | desk-invented parent references ($a, @1, ref a) | `'x = a{ b:2 c:{ parent:$a } }; x.c.parent.b'	2` (test/parser/test_references.pairs.warp:4) | – |
| 6 | desk-invented `class int` values / C declarations | `'class int == int'	true` (test/types/test_bare_class.pairs.warp:4) | – |
| 5 | e = error(…) raises, only a try or e: error? keeps it (decision errors-one-rule) | `'e = error("bad"); e.message'	'bad'` (test/control/test_error_values.pairs.warp:6) | – |
| 5 | desk-invented `map square on xs` | `'square:=it*it; map square on [1 2 3]'	[1 4 9]` (test/functions/test_map_phrase.pairs.warp:4) | – |
| 5 | typed declarations without a value (pixel:int[100], x : 10 float) | `'pixel:int[100]; pixel#1'	0` (test/types/test_typed_arrays.pairs.warp:7) | – |
| 4 | running data as code at run time needs a run-time interpreter | `'x = code 1+2; x!'	3` (test/control/test_block_parameters.pairs.warp:4) | – |
| 4 | effects of f / f.effects | `is! 'print "a"` (test/functions/test_effects_value.warp:4) | – |
| 4 | `maybe x` parameters | `'def f(maybe x){ x ?? 5 }; f() + f(2)'	7` (test/functions/test_optional_parameters.pairs.warp:17) | – |
| 4 | functions equal by extension | `'xs = [x => x + 1, y => y + 1]; xs#1 == xs#2'	true` (test/functions/test_runtime_function_equality.pairs.warp:7) | – |
| 4 | one-parameter function given several arguments maps them (f 1 2 3) | `'sum := fold +; sum 1 2 3'	6` (test/functions/test_several_arguments.pairs.warp:4) | – |
| 4 | nop word | `'x=0;while x++<11: nop;x'	11` (test/lists/test_lists.pairs.warp:39) | – |
| 4 | require/import/use of square, a module that does not exist | `'require square; square 7'	49` (test/modules/test_include.pairs.warp:12) | – |
| 4 | log operator ↓ | `'8↓2'	3` (test/numbers/test_arrow_operators.pairs.warp:4) | – |
| 4 | desk-invented dashed keys `a-b:2 … a-b` | `'a-b:2 c-d:4 a-b'	2` (test/numbers/test_math.pairs.warp:17) | – |
| 4 | desk-invented ordinal phrasings (char #1 in t, 2nd char in t) | `«char #1 in 'world'»	'w'` (test/text/test_string.pairs.warp:56) | – |
| 4 | C-style declaration with a zero value (int n;) | `'real x; x*x'	0` (test/types/test_declaration_zero_value.pairs.warp:4) | – |
| 4 | C/Java-style declarations (int bar(){…}, namespace, class Foo;) | `'class Foo; int bar(){ 42 }; int boom(){ -1 }; Foo().boom()'	-1` (test/types/test_file_declarations.pairs.warp:4) | – |
| 4 | Java type words double/long as functions | `'double 4'	4.0` (test/types/test_type_word_user_function.pairs.warp:8) | – |
| 4 | Go/Java/JS spelling (func, String(…), as long, def … as string) | `'func add1(x int) int { return x + 1 };add1(41)'	42` (test/types/test_types.pairs.warp:4) | – |
| 4 | and/or/then as value operators on ints | `'fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)'	13` (test/wasm/test_wasm.pairs.warp:32) | – |
| 4 | if as a function call | `'if({2},{3},{4})'	3` (test/wasm/test_wasm.pairs.warp:303) | – |
| 3 | desk-invented loop heads (for (it%2==0) in xs:, all xs > 2: …) | `is! 'numbers=[1,5,3]; s=0; all numbers > 2: s += it` (test/control/test_all_with_condition.warp:4) | – |
| 3 | matrix index notation (1, 2; 3, 4)[1,0] | `'(1, 2; 3, 4)[1][0]'	3` (test/control/test_blocks.pairs.warp:8) | – |
| 3 | desk-invented colon loops (each xs: …, all xs: …, 1..4 do …) | `'s = 0; each [1,2,3]: s += it; s'	6` (test/control/test_each_colon.pairs.warp:4) | – |
| 3 | undo word | `is! 'stored kept_undo = 1; kept_undo = 2; kept_undo = 3; undo kept_undo; kept_undo' 2` (test/control/test_undo_history.warp:4) | – |
| 3 | a block applied to a list by brackets, {it*it}[1 2 3] | `'({it^2}[1 2 3])#3'	9` (test/functions/test_anonymous_application.pairs.warp:8) | – |
| 3 | forEach/for_each/foreach spellings | `'total = 0; [1,2,3].forEach { total += it }; total'	6` (test/functions/test_each_spellings.pairs.warp:4) | – |
| 3 | effect annotations `! Pure`, `! Fail` | `is! 'f(x) := 10/x ! Fail` (test/functions/test_fail_effect.warp:4) | – |
| 3 | a block called with named values, `{x+y}(x=1, y=2)` | `'{x*x}(x=5)'	25` (test/functions/test_lambdas.pairs.warp:14) | SPECIFICATION.md, symbolism.md |
| 3 | `use square`, a module that does not exist | `'use square;square 7'	49` (test/modules/test_use_modules.pairs.warp:4) | – |
| 3 | truthiness under ≈ (yes ≈ 2, no ≈ "") | `'yes ≈ 2'	true  # keep but later extension` (test/operators/test_approximately.pairs.warp:19) | – |
| 3 | circa/approximately word synonyms of ≈ | `'1 circa 1.0000000001'	true` (test/parser/test_surface_syntax.pairs.warp:32) | – |
| 3 | Java/C type spellings (String, double) | `'π as String'	'π'` (test/text/test_text_type_words.pairs.warp:7) | – |
| 3 | a bool into an int variable and back | `'n: int = true; n + 1'	2` (test/types/test_bool_assign.pairs.warp:8) | – |
| 3 | C-style declaration without a value (real x;) | `'real x; x*x'	0` (test/types/test_declaration_without_value.pairs.warp:4) | – |
| 3 | postfix type word, `xs:float list` | `'xs:float list = [0.5 as float, 0.25 as float]; f(j) := xs#j; f(1)'	0.5` (test/types/test_list_annotations.pairs.warp:4) | – |
| 3 | `maybe int` type spelling | `'x: maybe int = 1; x'	1` (test/types/test_maybe_declaration.pairs.warp:4) | – |
| 3 | a type word after a size makes an array, `surface=height*width int` | `'height=4; width=4; surface=height*width int; #surface'	16` (test/types/test_sized_array_values.pairs.warp:4) | – |
| 3 | unknown names as data, `cat{x=1 y=2}` | `'cat{x=1 y=2}; cat.x + cat.y'	3` (test/types/test_symbolism_object.pairs.warp:4) | – |
| 3 | a declaration x as number = 9 | `'x as number = 9; x'	9` (test/types/test_type_tests.pairs.warp:41) | – |
| 3 | unknown name as 0, `i*=3`, `test42+1` | `'i*=3'	0` (test/wasm/test_wasm.pairs.warp:235) | – |
| 3 | ¬ on ints as 0/1 | `'¬ 1'	0` (test/wasm/test_wasm.pairs.warp:264) | – |
| 3 | assigning the constant ℯ | `'use log;ℯ = 2.7182818284590;ln(ℯ)'	1.0` (test/wasm/test_wasm.pairs.warp:456) | – |
| 2 | desk-invented `expect 30 10+20` | `'expect 30 10+20'	ø` (test/control/test_expect.pairs.warp:4) | keyword.md |
| 2 | desk-invented filter loop heads for (it>2) in xs | `'xs=[1,5,3]; s=0; for (it>2) in xs { s += it }; s'	8` (test/control/test_filter_loop_over_variable.pairs.warp:4) | – |
| 2 | lambda spelling (a b)->a+b | `'fold [1 2 3] 10 (a b)->a+b'	16` (test/control/test_iteration_words.pairs.warp:11) | – |
| 2 | was/before, synonyms of old in a change handler | `'x = 1; d = 0; on change x {d = value - was}; x = 5; d'	4` (test/control/test_reactive_ports.pairs.warp:26) | – |
| 2 | reflection synonyms (attributes, members), one word each is enough | `is! 'class P{x:int; y:int; sq() := x*x+y*y}; p = P(1, 2); p.attributes' ['x', 'y']` (test/control/test_reflection_words.warp:4) | – |
| 2 | Python def stop(): … with after stopped / before save: | `is! 'n=0; def stop(): 1; after stopped {n+=1}; for i in 1 to 3 { stop() }; n' 3` (test/control/test_variable_signals.warp:13) | – |
| 2 | invocable/runnable, synonyms of callable | `is! 'run(f: invocable) := f()` (test/functions/test_callable_parameter.warp:23) | – |
| 2 | old wasp definition by space and it beside a parameter | `'twice x := it * 2; twice 4'	8` (test/functions/test_definition_value.pairs.warp:10) | – |
| 2 | desk-invented count phrasings (number of bytes in t, .size in bytes) | `'"äb".size in bytes'	3` (test/functions/test_member_phrases.pairs.warp:13) | – |
| 2 | $0 parameter / lambda spelling x -> … | `'[[1,2],[3]].map{ $0.map{ $0*10 } }'	[[10 20] [30]]` (test/functions/test_nested_it.pairs.warp:16) | – |
| 2 | desk-invented p's name, square of a number = … | `«p={name:"Joe"}; p's name»	'Joe'` (test/functions/test_property_access.pairs.warp:14) | – |
| 2 | desk-invented phrase definition `to render text as pdf:` | `is! 'class pdf{body:text}; to render text as pdf: pdf(text); x = (render "hello") as pdf; ` (test/functions/test_render_as.warp:4) | – |
| 2 | semicolons separate statements, the group is its last value | `'(1;2;3)'	[1, 2, 3]` (test/lists/test_lists.pairs.warp:60) | list.md |
| 2 | Haskell where binding, $0/$1 | `'x * 2 where x = 3'	6` (test/lists/test_where_filter.pairs.warp:12) | – |
| 2 | require / effects of main | `is! 'require math` (test/modules/test_import_words.warp:12) | – |
| 2 | `law` property declarations | `'add(a,b) := a+b; law add(a,b) == add(b,a); add(3,4)'	7` (test/numbers/test_law.pairs.warp:4) | – |
| 2 | `use sin`, a module named after a function | `'use sin;sin π/2'	1` (test/numbers/test_math.pairs.warp:35) | – |
| 2 | desk-invented unit prefix/name declarations (prefix x := 10^4, name furlong := fur) | `is! 'prefix x := 10^4` (test/numbers/test_unit_prefix_lines.warp:4) | – |
| 2 | desk-invented `x be 3` | `'x be 3; x'	3` (test/parser/test_surface_syntax.pairs.warp:21) | – |
| 2 | $0 parameter / pixels.number() | `'add1(x):=$0+1;add1(3)'	4` (test/sweeps/test_todo.pairs.warp:10) | – |
| 2 | desk-invented `x be number 9` | `'x be number 9; x+1'	10` (test/types/test_be_definitions.pairs.warp:4) | – |
| 2 | def f(x) as text {…}, a return type after as | `'def f(x) as text { 3 }; f(2)'	'3'` (test/types/test_checked_results.pairs.warp:8) | – |
| 2 | collection declarations tuple t = …, array xs = … | `'tuple t = (1, 2, 3); t#3'	3` (test/types/test_collection_declarations.pairs.warp:6) | – |
| 2 | C-style declarations (int n, namespace, int bar(){…}) | `is! 'int n` (test/types/test_declaration_without_value.warp:4) | – |
| 2 | desk-invented `x be …` | `'x be number 3; x * 3'	9` (test/types/test_is_declaration.pairs.warp:4) | – |
| 2 | record keyword | `'record point{x:int y:int}; point(1,2).y'	2` (test/types/test_records_classes.pairs.warp:4) | – |
| 2 | postfix type word, `xs: int list` | `'f(xs: int list) := xs#2; f([3 4])'	4` (test/types/test_trait_constraints.pairs.warp:4) | – |
| 2 | text as bool | `«'2' as bool»	true` (test/types/test_types.pairs.warp:23) | – |
| 2 | Python mutable default argument with a (0 0) group | `'def f(a=(0 0)){ a#1 = a#1 + 1; a#1 }; f()+f()'	2` (test/welcoming/test_footguns.pairs.warp:283) | Footguns.md |
| 1 | warn, a synonym of warning | `'"a" + warn("careful") + "b"'	'ab'` (test/control/test_error_words.pairs.warp:9) | – |
| 1 | desk-invented `listeners of alarm -= 2` | `is! 'n = 0; on alarm {n += 1}; on alarm {n += 10}; listeners of alarm -= 2; emit alarm; n'` (test/control/test_event_handler_lists.warp:9) | – |
| 1 | unknown names as symbols / type-word loop for int in xs | `'x = hello; print x; 1'	1` (test/control/test_for_it.pairs.warp:7) | – |
| 1 | desk-invented unit loop heads (for bytes in t:) | `'t = "héllo"; n = 0; for bytes in t: n += 1; n'	6` (test/control/test_for_unit_words.pairs.warp:5) | – |
| 1 | returns, a synonym of return_type | `'sqrt.returns'	'number'` (test/control/test_function_value_words.pairs.warp:5) | – |
| 1 | previous, a synonym of old in a change handler | `is! 'x = 1; log = 0; on change x {log = log*10 + previous}; x = 2; x = 3; log' 12` (test/control/test_reactive_ports.warp:26) | – |
| 1 | after-participle-held | `«n=0; def test(x): x+1; after tested: n+=10; test(1); test(2); n»	20` (test/control/test_variable_signals.pairs.warp:18) | – |
| 1 | lambda spelling x -> x*2 | `'f := x -> x*2; f(3)'	6` (test/functions/test_arrow_result_type.pairs.warp:11) | – |
| 1 | old wasp definition by space, `fibonacci number := …` | `is! 'fibonacci number := if number<2 : 1 else fibonacci(number - 1) + fibonacci it - 2; fi` (test/functions/test_definition_value.warp:4) | – |
| 1 | $0/$1 parameters | `'g(a, b) := { $0 - $1 }; g(5, 2)'	3` (test/functions/test_dollar_body.pairs.warp:4) | – |
| 1 | effect annotations `! Pure` | `is! 'wave(x) := exp(x) ! Pure` (test/functions/test_libm_pure.warp:4) | – |
| 1 | old wasp definition by space, `f y := …` | `'f y := y*y+v; f(y=2, v=3)'	7` (test/functions/test_named_arguments.pairs.warp:13) | – |
| 1 | a block called with named values, `{x*y}(x:2 y:3)` | `'g={x*y}; g(x:2 y:3)'	6` (test/functions/test_named_arguments.pairs.warp:14) | – |
| 1 | desk-invented phrase definition `to boop x:` | `'to boop x: x + 1; to boop x for y: x + y; x = {boop 2}; count(x)'	2` (test/functions/test_statement_blocks.pairs.warp:4) | – |
| 1 | C-style typed definition float twice(x) | `'float twice(x) := {x+x}; twice 2'	4.0` (test/functions/test_typed_returns.pairs.warp:11) | – |
| 1 | `name`/`prefix` as declaration words for plain functions | `'name p := p.first; name {first: "Bo"}'	'Bo'` (test/numbers/test_unit_prefix_lines.pairs.warp:16) | – |
| 1 | `prefix` operator as declaration words for plain functions | `'prefix s := s#1; prefix "ab"'	'a'   todo: only allow prefix operator as keyword else ambi` (test/numbers/test_unit_prefix_lines.pairs.warp:18) | – |
| 1 | typed declaration without a value, `x : int[100000]` | `'x : int[100000]; count(x)'	100000` (test/numbers/test_zero_fill.pairs.warp:6) | – |
| 1 | desk-invented rough_tolerance | `'rough_tolerance = 0.1; 1 ~ 1.05'	true` (test/operators/test_rough_similarity.pairs.warp:9) | – |
| 1 | log operator ⬇ | `'8 ⬇ 2'	3` (test/parser/test_operator_aliases.pairs.warp:18) | – |
| 1 | display synonyms density/ppi | `'display.density == display.dpi and display.ppi == display.dpi'	true` (test/programs/test_display_names.pairs.warp:4) | – |
| 1 | desk-invented `do add 4 to pixel` | `'pixel=[1 2 3]; do add 4 to pixel; pixel'	[1, 2, 3, 4]` (test/sweeps/test_sweep_fixes.pairs.warp:8) | – |
| 1 | s++ on a text | `'s = "a"; s++; s'	'a1'` (test/types/test_chained_assignments.pairs.warp:10) | – |
| 1 | clone, a synonym of copy | `'class P { x: int }; p = P(1); q = p.clone(); q.x = 2; q.x'	2` (test/types/test_copies.pairs.warp:7) | – |
| 1 | dashed field names, phone-number | `is! 'class person{phone-number:text}; p = person{phone-number:"12"}; p.phone-number = "3";` (test/types/test_struct_instances.warp:25) | – |
| 1 | implicit multiplication of a vulgar fraction, ⅓x | `'x=9;⅓x'	3` (test/wasm/test_wasm.pairs.warp:4) | – |
| 1 | C-style typed declaration `int i=…` | `'int i=π*1000000'	3141592` (test/wasm/test_wasm.pairs.warp:373) | – |
| 1 | for i=1..3; loop head | `'sum=0;for i=1..3;sum+=i;sum'	6` (test/wasm/test_wasm.pairs.warp:473) | – |
| 1 | require, a synonym of use | `is! 'require •/test/fixtures/wasm/fourty_two;ft*2' (42 * 2)` (test/wasm/test_wasm.warp:48) | – |
| 1 | pins i64 wraparound, a flaw | `'f(x) := (x*x) as i64; f(3037000500)'	-9223372036709301616` (test/welcoming/test_footguns.pairs.warp:39) | – |
| 1 | desk-invented add(…, overflow: clamp) | `'add(1 month, overflow: clamp) == 2024-02-29'	true` (test/welcoming/test_footguns.pairs.warp:249) | – |
| 1 | fast floats (`fast` keyword reassociating float math) | `# 'fast a=0.1; fast b=0.2; fast c=0.3; (a+b)+c == a+(b+c)'	false # DON'T EVER PIN 'BAD BEH` (test/welcoming/test_footguns.pairs.warp:375) | – |
| 1 | C float32 literal suffix .1f | `# '.1f + .2f == .3'	false` (test/welcoming/test_footguns.pairs.warp:388) | – |
| 1 | symbolic trig: sin(π/6) exactly 1/2 | `# triage-held 2026-11-06: symbolic real equalities (sin(π/6) == 1/2, ℯ^(ⅈπ) == -1, surds) ` (test/welcoming/test_footguns.pairs.warp:408) | – |
| 1 | symbolic real equalities (sin(π/6) == 1/2, ℯ^(ⅈπ) == -1, surds) | `'sin(π/6) == 1/2'	true` (test/welcoming/test_footguns.pairs.warp:409) | Footguns.md |
| 1 | renamed-params function equality (user hold) | `# is! 'fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fibo(m) = m < 2 ? m : fibo(m - 1) + f` (test/welcoming/test_footguns.warp:37) | – |

## Batch 3 (card test-triage)

Applied the Interviewer's batch 3. Restored: `!==` (green), `x be …` (red, card says-unknown), circa/approximately (red,
card circa-approximately), `⅓x` (red, card fraction-literal-bug). Rewritten: `(1;2;3)` and `{1;2;3}` give 3; stored
errors are declared `e: error? = error(…)` (errors-one-rule, green). Deleted: i64 wraparound, unknown names as 0 or
data, modules that don't exist, assigning ℯ, s++ on a text, if as a call, matrix index notation, a block applied by
brackets, mapping by extra arguments, `x as number = 9`, `for i=1..3;`, `$0`/`$1` lines, `def stop(): … after
stopped`, rough_tolerance, text as bool, ¬ on ints (test_symbolism_object.pairs.warp had nothing left). Kept, by a wiki
hit: the desk-invented loop heads, colon loops and filter loop heads (for.md:47 `for (it>2) in …`, iteration.md:59
`each [1,2,3]: …`). Still open, with the user: the synonym words, wasp definition by space, typed declarations without
a value, a return type after `as`, undo/redo, and/or as value operators, `n: int = true`, postfix type words.

## Batch 4 (card test-triage)

The user's decisions synonyms-are-aliases, definition-by-space, typed-array-declaration and result-type-as: their held
lines are restored (139), rough_tolerance is back (operator.md:41). The synonyms are prelude alias lines
(lib/prelude.warp: description, returns, was, previous, invocable, runnable, forEach, for_each, foreach, integer,
string, unit, nil), not lib/aliases.warp entries: that table aliases operator and keyword spellings and changed none of
the lines. A prelude alias reaches only a library call (`integer(3.7)`), a program's own alias line every use, so
101 restored lines stay red, each group carded: prelude-alias-line (the mechanism, a core change for the Interviewer),
square-add1-twice (definition by space), array-type-sizing, def-text (result types after `as`), maybe-int-refused, type-unit, twice-representation-fs,
string-concat-warn, display-density, the reflection words (card reflection-dot-forms: p.fields
itself is not built). Held still, outside the decisions' wording: `exact == real`, `numbers x = [1 2]`,
`def f(x:int) -> int or text`, `f(x) := x + it`, `require math` with `effects of main`; `add1 x:=$0+1` deleted ($0).

## Update after the sample fixes (run 16:53 at 99d8f7f7a1: 1422 of 15481 failed, 2438 files)

The next run was still going, so this counts the 16:53 results: 1024 errors, 398
wrong values. The clusters below come from the error text, grouped (script: the error without position and `(no owner)`). Cards
landed since that run: unknown-name-test, package-tests-registry (test_packages:4 green), library-typed-outcome.

Biggest clusters by cause (new or grown against the earlier triage in **bold**):

| cases | cluster (error) | main files | card |
|--|--|--|--|
| 398 | wrong value, no common error: regex_malformed 9, code_type 8, one_dispatch 8, quasi_quote 6, big_int_bits 6, hyperreals 6, meta_attributes 6 | judge per file | (per file) |
| 40 | wasm trap: cast failure | test_nearest 12, sound_spectrum 4, try_list_index 3, nearest_default 3 | cast-failure-traps |
| 21 | `text_with_at` (text element write) | test_string 12, footguns 6, test_wasm 3 | triage-text-element-write |
| 20 | a function value in `.` (charged fields) | charged_fields 5, object_functions 4, reflection_mode 4, on_error_of_function 3 | charged-field-values |
| 20 | `x!` / `interpret(…)`: running a node as code (14 + 6 unknown interpret) | run_time_blocks, run_time_block_cache, object_blocks | fails-outside |
| **18** | **not a list: `map reverse [[1 2] [3 4]]`, a library word mapped** | map_library_word_first, parameter_scope 3, path_after_function 3, property_definitions 3 | sorted-map-library |
| **12** | **not a function: `twice(body: block) := { body!; body! }`, a block parameter run** | block_parameter_calls, block_parameters, function_objects, count_shadowed | block-values-forced |
| **12** | **unknown name a: a lambda's own parameter or a sibling field, `(a: int, b = 1): int => a + b`, `{a: 3, f: x => x * a}`** | field_function_values, parameter_patterns, block_data_paths, arrow_defaults_and_types | lambda-own (new) |
| 12 | operator . on int (`it` in nested blocks) + 11 unknown name it | nested_it 4, returned_blocks 4, versions 3, glued_method_block 3 | nested-it-blocks |
| **12** | **`xs.insert(4, at: 0)`: insert·list·int·any called with 3 arguments** (was 5) | welcoming_sugar 5, position_meets_brackets 2, list_insert | library-labelled-args |
| 11 | a variable declared fast | fast_scope 7, fast_int 3 | fast-declaration |
| 10 | call stack exhausted | try_stack_overflow 3, numeric 2 | add-stack-overflow |
| 10 | invalid number (`format 2.2 as int` pick) | argument_cast 4, exponent_texts 3 | argument-cast-overload |
| 13 | `session[…]` / `use storage` words | test_storage, store_values | session-storage-words |
| 10 | `•/test/fixtures/wasm/*.wasm`: no folder above . holds .git | wasm_modules, test_wasm, component_short_use | wasm-fixture-root |
| **6** | **`sorted(xs, >)`, `.sort(by: -it)`: sorted_node called with 2 arguments** | relational_arms, it_arguments, named_function_argument, operator_arguments | sorted-comparator-syntax (new) |
| **6** | **no case for value = 0: `match v { Circle => …; Square => … }` over classes** | class_destructuring 3, match_type_arms, structural_patterns | tree-patterns? |

Unchanged since the earlier triage: ∞ 7, divide by zero 7, `x as text?` 6, Color cases 6, calendar/clock durations 6,
`time` defined twice 6, callable parameter 6, tuple returns 5, mixins 5, Array.from 5, display 5.

## numbers n–z (cloud worker numbers-n-z-lib, 2026-11-10, main 51b1e68)

`warp test test/numbers/test_[n-z]*`: 61 of 627 ✗. No cluster has its cause in lib/ alone, so nothing changed.

| lines | cluster | root cause | smallest shape |
|---|---|---|---|
| 16 | unit spellings as literals: `3 kilometers`, `str(3 meter)`, `1 Gs`, `1000 µg`, `1 dam`, `1 kK`, `1 Mg`, `1 mph`, `2dot`, `3 feet in inches` (unit_literals 4-6 11, unit_prefix_lines 4-6 9 13, unit_definitions 10 22 24 35, pixel_unit 10) | a unit word is a name lib/units declares as a `Unit` (rule_words is_unit_word); lib/unit_names reads every spelling, but only at run time (`quantity(3, "kilometers")` works). Declaring each spelling would repeat unit_names' tables | the rule engine reads an unbound word after an amount as a unit when unit_names knows its spelling (a module text word callable in a `where`), card headless-module-rules; or `$u is unbound` → `quantity($e, $name)` with a run-time refusal (question below) |
| 8 | a program variable named like a unit: `m = 5; str(3m)` "3m", `s = 3; str(0.5s)`, `h = 2 inch; 3 km/h`, `s = 2; 9.8 m/s²` (unit_beside_variable 4 + .warp 4, units_after_numbers 4 7 16 20, units_arithmetic 4) | core/passes/rewrites.warp typed_as: a name the program binds hides the library's Unit, so glued `3m` multiplies the program's m; the glued literal is not told apart from `3*m` | the parser marks a glued number+word, and `$e * $u` of a glued one takes the library's unit |
| 7 | a spaced unit inside `if … :`, `if … {` or a chain of comparisons: `x = 2 km; if x > 1500 m : …` (DimensionError 1500 plain), `2 h + 30 min == 150 min`, `now < 24h`, `now >= 0 ms and now < 1 day`, `if now < 0 h` ("0 what?") (time_of_day 4 5 6 14, unit_literals 9 10, unit_word_keys 13) | extensions/rules/rule_words.warp unit_bound walks only unit_binding_ops; the `if` head and a second unit in one comparison (card spaced-unit-comparison) leave the unit off its amount | unit_bound through the condition of if and through both sides of ==/< |
| 5 | `nearest` of an exact real (nearest 4, nearest.warp 4 5, nearest_default 8 9) | lib/prelude nearest's steps turn √2 into floats after the first (`1/(rest - a)` of a real is a float); limit 0 of an irrational loops (Error "") | lib-only once the tests agree: x to enough exact digits (lib/reals decimal_digits' scaled total), limit 0 of a non-float irrational its float's. **Conflict**: nearest:4 wants 941664/665857 (exact √2) but the passing nearest:21 `x = sqrt(2); nearest(x, rational) == 665857/470832` wants the float's; `x = sqrt(2)` is the exact √2 now, so fixing 4 turns 21 ✓ → ✗ |
| 3 | `type(3px)` "pixels", `type(2 m²)` "area", `type(5 Hz)` "frequency" (pixel_unit 5, unit_definitions 18 19) | type() shows the class, Pixels / Area / Frequency | question: lower-case dimension words canonical? |
| 2 | `-0.0/0.0`, `(0.0/0.0) * -1.0` divide by zero (nan_observed 4 5) | core float division refuses 0.0/0.0 | core |
| 2 | `1 << 100`, `(2^100) >> 98` (shift_operators 18 19) | shifts are i64 | core big_numbers |
| 3 | `unit furlong := 201.168m; 1km as furlong` (operator as on int), `14 * 24 h` refused, `x = 2 hand; x as cm` (unit_definitions 29, .warp 4 9 15) | card module-rule-unnamed (lib/units comment); `14 * 24 h` is the intended unit-after-product refusal (decision unit-after-number) → wrong expectation | |
| 2 | `unit rankine as kelvin := …`, `unit hand as inch := 4 * it` (unit_definitions.warp 19 21) | no `unit … as …` conversion form | esoteric? |
| singles | `x.ceil` no field (number_methods 6: core emitter single_own_words has floor, not ceil); `gcd(2^70, 3·2^65)` 0 (core gcd is i64; lib/math wide_gcd exists); `√(4 m²)` stack exhausted (Quantity has no root()); `sqrt(2) as rational`, `"2.5 km" as quantity`, `"1 km" as unit` (no conversions); `type([π 1])`, `list of rational` (core types); `x : int[100000]` (core); `v: km/h = 3 km/h` (core) | | |

Wrong expectations (approve to correct):
| file:line | expected now | proposed | why |
|---|---|---|---|
| test/numbers/test_rounding_in_functions.pairs.warp:6 | 2 | f(2.7) → 3 | decision round-halves: a half may round either way; tests never pin it |
| test/numbers/test_unbounded_int.pairs.warp:57 | "too big for int: 9223372037000250000" | "too big for int; fix: `x as! int` keeps its low 64 bits" | the refusal now names the fix-it (decision checked-forced-conversions) |
| test/numbers/test_unit_prefix_lines.pairs.warp:19 | `'a'   todo: …` (spaces, read as the expected) | a tab before the note | got "a" already; the note is glued to the expected value |
| test/numbers/test_units_after_numbers.warp:30 | `s = 3; str(440Hz)` "440/s" | "440Hz" | Hz shows as written (canonical-output); Rust-era form |
| test/numbers/test_unit_after_comma.pairs.warp:4 | `str([1, m])` "[1, 1m]" | "[1, m]" | a Unit shows by its symbol (lib/units Unit.text); esoteric |
| test/numbers/test_unit_definitions.warp:9 | `14 * 24 h` as a unit's value | `(14 * 24) h` | decision unit-after-number refuses a unit after a product |

## types n–z (worker types-n-z-lib, 2026-11-10, main 4a663896f0)

`warp test test/types/test_[n-z]*`: 167 of 1209 ✗ in 189 files. No cluster has its cause in lib/ alone, so nothing
changed; most test files are Rust-era (`# from test_….rs`) with no wiki page behind them.

| lines | cluster | root cause | smallest shape |
|---|---|---|---|
| 11 | union return types: `def f(x:int) as int or text { … }`, `as int \| text`, `as Circle or Square` (union_returns 4-14, union_spellings 4) | the analyzer reads no union after `as` in a def head: "no function f", "an assignment to `… or …`" | the def head takes a type expression after `as`, as a parameter's type does |
| 18 | traits and protocols: `perimeter()` default methods, `Comparable`, `is shape`, `xs: shape list` (protocols 3-17, traits 4-11, trait_constraints 4-9, trait_default_methods 4-12, trait_equals_membership 4 6) | a protocol's default methods are not given to its classes; Comparable is no declared trait (lib could declare it, but `x is Comparable` needs structural tests in core) | card: protocol default methods + structural `is`; Comparable/equals as lib traits once that holds |
| 15 | times as types: `type(now.time)` "data", `type(2024-06-01)` / `type(1 day)` no owner, `time.now` no field, `now > 1970-01-01` "time of node", `str(…T22:27.time)` "no length of symbol", `type(now) == instant`, `.tzdata` (time_now 4-20, time_type_names 6-9, partial_time_comparisons 10-12, time_kinds_are_types 11 14, time_programs_run 4) | the emitter's type of a time literal has no owner; a date literal compared with a lib Instant is no Instant; `.time` of a literal is read as a symbol | the time literal lowers to lib/time's value (decision time-word), so type() and comparisons are lib's |
| 9 | typed sized arrays: `x : 10 float`, `surface = 16 int`, `x : n*2 float`, `if 0 { x = int[3] }` (sized_array_values 4-6, typed_arrays 11 12 21) | `amount type` is no sized-array form in core; `16 int` reads as a list of two | esoteric? (`int[16]` exists); Interviewer |
| 8 | `text?` / `int?` as a cast target: `x as text?` (optional_casts 4-16) | the unknown name `text?`: `as` takes no optional type | `as T?` = `as T` letting ø through: core cast |
| 8 | variant types: `c.0` of `rgb(1,2,3)`, `red is Color`, `Option[T]` (variant_payloads 4-24) | a test of a type with cases, `.0` of a payload: no core form | card |
| 9 | function types shown: `type of f` "function: (x) -> int", `type(int)` "type" (type_of_consistency 4 5 8, type_of_lambda_variable 4-6, type_values 11) | type() of a function shows "function" only | Interviewer: is the signature wanted in type()? |
| 8 | type calls as casts: `hex(123)` 123 (gets the Color 0xFF00007B), `decimal(1/4)`, `byte(65)`, `char(0x41)`, `text(hex(255))` (type_calls 4-14, text_of_text 10, types 34) | `T(x)` ≡ `x as T` is no core rule; lib/color's hex takes the call | Interviewer: `T(x)` as the cast, a general rule; `char(code)` alone could be a prelude function |
| 7 | one dispatch: `p.first()` of a map field, `width(canvas)`, `first(x) := …` shadowing the library's, setter `set w(v)` (one_dispatch 5-87) | a program's own definition and a field call read the same; mixed cases | split: each line a decision (decision one-dispatch?) |
| 6 | records with untyped fields: `class contact {name email}; contact("Joe" "j@x")` (records_classes 4-16) | a class of bare names takes a list as its first field | unpinned: f(a b) with several parameters, whatever the compiler does (user 11-13 via Interviewer); the lines dropped |
| 6 | a mutating method gives the changed object: `class counter{n:int; inc() := n += 1}; counter(4).inc().n` (mutating_methods pairs 4, warp 9) | a method gives its body's value, the new n | unpinned: a method gives its body's value; chaining writes `inc() := { n += 1; self }` (user 11-14 via Interviewer, answer-or-act); the lines dropped |
| 5 | typed JSON: init effects and guards (typed_parse_constructor_effect 2-4, _guard 2, _flat_guards 2) | extensions/rules/rule_words.warp:1385 refuses or skips | extension |
| 4 | `nat`: `type(3)` "nat", `type(x) == nat` (type_static 9, type_values 7 15, type_compared_with_text 5) | int is canonical now | wrong expectation? Interviewer |
| 3 | `gc struct` types (recursive_fields 8 9 12) | wasm-level struct spelling | esoteric |
| 3 | months as values: `now >= January` (partial_time_comparisons 4-6) | no Month vocabulary in lib/time (MONTH_NAMES is a list) | lib enum Month + an Instant compared with a Month: Interviewer first (Rust-era, no wiki) |
| 3 | `print` with a class's `text()` in a list: `str([N(1)])` "[n1]" (printable_items 4 7 21, printable 5) | a list shows its items by the generic form, not their text() | core show |
| 3 | `parse("1+1") is operation` (operation_type 14 15 17) | no operation/expression types | esoteric? |
| 3 | `ints`/`texts`/`a list of int` (type_tests 21 25 26) | plural type words unknown in `is` | alias rule; Interviewer |
| singles | `f 3 == 6` refused as two readings (nested_type_calls 15: the refusal is right, refuse-never-guess → wrong expectation); `P(1/3).v is float` no (number_converts 4 7 9); `x.type` (type_of 4); `t: Type` (type_parameters 6 21); `list of int16` (types 50, types_scope 12); `'2.1' as real` (types 17); `2^70` in an int field (struct_instances 16); `inc(c)` copy (struct_instances 18); `string(p)` spacing (struct_instances 13: "Point{x:6 y:4}" is Rust-era → wrong expectation); `form post "/todos"` clashes with lib/net post (symbol_parameters 4 9); `friends{ div(…) }.div` (tag_children 4); `cat{ x = 1 "t" }` (tag_entries 4); `insert.params` order (typed_builtins 16); `{x:1 y:2} as Point` (unknown_conversion 6); `list of any` (unknown_type 9); `match` on an upcast (upcast_fields 6); `value{…}` block (value_constructor 4); `π is number` no (type_tests 30); `str(1 day in h)` (time_programs_run 7); `shown` overload (units_free_function 2); empty() maps (plain_enum 17 18); fibonacci number (type_name_matching 4); string(result) spacing (one_dispatch 29) | | |

## lists n–z (worker lists-n-z-lib, 2026-11-10, main aa75297d68)

`warp test test/lists/test_[n-z]*`: 18 of 393 ✗ in 65 files. No cluster has its cause in lib/ alone.

| lines | cluster | root cause | smallest shape |
|---|---|---|---|
| 5 | named fields of a bracket list: `p = [x:1, y:2]; p.x`, `f(p) := p.xy` (swizzles 21-25) | `.name` on a list of pairs is no field read (a map's is) | core: `.name` of a list of pairs reads its pair, as `{…}` does |
| 5 | unpacking: `a, b = f()` of `return 1, 2`, `(a, b) = 1, 2`, `(a, b), c = …`, `_, b = f()`, `for a, b in [[1,2],…]` (unpacking 16 28 50 56 59) | the assignment of a list to a pattern only takes plain names | core destructuring pass |
| 3 | a comprehension inside `where`, or `to#i` in its filter: `#[i for i in idx where to#i == 0]` (nested_comprehension 7 8 14) | the comprehension's name is unbound in its `where` when the filter indexes another list | core/extension comprehension scope |
| singles | `sqrt (1 to 4)` exact `(sqrt 2)` where the test wants floats (range_broadcast 2: wrong expectation, user 11-11: exact reals are right); `{r = 1..100001}; o.r` no field (range_in_structure 10); `f(xs) := {xs#1 = 9; xs#1}; f(1..5000)` gives 1 (range_arguments 16: a range argument is not a list to write into); `qs(xs…)` sorts the caller's xs in place (node_lists 9: user 11-11: a function may change the caller's list but must be named with `!`; card param-mutation-rules, the line then calls qs! or expects the refusal); `values(pair)` "not a map" (pair_values 4: values of a pair is its value?) | | |

### types n–z: the user's verdicts (2026-11-11, via the Interviewer)

- Cards: union-return-types, protocol-default-methods (with structural `is shape`, Comparable), optional-cast-text,
  time-literal-values, variant-payloads, untyped-record-fields, type-call-cast (`T(x)` as the cast; hex's meaning is
  decided on that card; `char(code)` may be a prelude function now).
- Held, not cards: typed JSON init effects (typed_parse_constructor_effect 2-4, _guard 2, _flat_guards 2); a list
  showing each item's own text() (printable_items 4 7 21, printable 5).
- Held as esoteric: months as values (partial_time_comparisons 4-6), `type(f)` with a signature (type_of_consistency,
  type_of_lambda_variable, type_values 11), sized `x : 10 float` / `16 int` (sized_array_values, typed_arrays 11 12 21),
  `gc struct` (recursive_fields 8 9 12), `parse(…) is operation` (operation_type 14 15 17), plural type words
  `[1 2] is ints` (type_tests 21 25 26).
- Wrong expectation, parked (left red): `type(3)` "nat" (type_static 9, type_values 7 15, type_compared_with_text 5),
  `f 3 == 6` refused as two readings (nested_type_calls 15), "Point{x:6 y:4}" (struct_instances 13).
## test/control/test_[n-z]* (cloud worker control-n-z-lib, main 51b1e68)

166 of 848 failed in 187 files. No lib-only cluster of 5+ lines: every cluster below needs core/ or extensions/, so the
run was report-only (nothing changed in lib/).

| cases | cluster | root cause (layer) | smallest shape |
|--|--|--|--|
| 21 | `interpret x` / `x!` / `w()` of data reading the caller's names (run_time_blocks 13, run_time_block_cache 6, uncharged_blocks 3, object_blocks 2) | lib/interpret.warp runs `str(x)` as a program of its own (the caller-names rule parked, card x-bang-sweep); exact and big numbers lose their kind through the text (`7/2`, `2^70`) | extensions: `interpret $x` passes the names free in `$x` as `eval(…, with: {…})`, values not reprinted as text |
| 16 | signal ports (reactive_ports 12, variable_signals, when_signals, signal_fields, time_of_day) | `on set p.age` (field ports) is no rule and a field write notifies nobody (emitter); `on change square` of a derived `:=` is refused by lib/signals.warp's `not $x is function`; `was`/`previous`/`before` as the handler's old value: a prelude alias reaches only calls | card prelude-alias-line (decision aliases-unified) for was/previous; a field-write notify hook in the emitter |
| 13 | reflection synonyms and dot forms (reflection_words, reflection_synonyms, value_methods) | `p.properties`/`p.attributes`/`p.members`: prelude aliases reach no dot form; a run-time `fields` cannot be a library word: rule_words.warp `owns_word("fields", program)` counts library words reached as the program's own, so `fields(P)` / `P.fields` of a class turn red (tried: a prelude `fields`, and `alias fields => field_names_of`, both ✓→✗ on test_reflection_synonyms.warp:10 and test_reflection_words.pairs.warp:17) | card prelude-alias-line; `owns_word` tests the program's own definitions only |
| 8 | `import •/test/fixtures/wasm/shapes` / `fourty_two`: "not a list", unknown name | extensions/c_imports/wasm_modules.warp | card wasm-fixture-root |
| 6 | `switch v {1: x=5 2: x=7}`: the call pattern_bindings(…) has no owner | the switch lowering asks for the prelude matcher after the prelude's words were reached (core/passes/node_matches.warp); `match` with the same arms works | the switch rule names pattern_bindings as reached |
| 10 | try around a trap or an exit (try_exits_and_naming 4, try_stack_overflow 3, try_named_traps 2, try_index_and_float) | the emitter refuses `#` on int / `.a` of int / return-break-continue out of a try at compile time instead of a catchable run-time trap | cards add-stack-overflow, cast-failure-traps |
| 6 | events (reflection_words 3, reactive_ports 3): `alarm.listeners`, `clear listeners of alarm`, `h = on alarm {…}` then `listeners of alarm -= h` | extensions/sugar/events.warp has only `listeners of $e` | one `$e.listeners` rule; `clear` as `unlisten` of all |
| 6 | shared values across tasks (shared_values 3, shared_signals 2, `atomic`) | a task's write to a `shared` global does not reach the caller | threads, core |
| 5 | reflection mode `twice.line`, `.origin`, `.wasm_name` | "a function value in ." (emitter) | card charged-field-values |
| 5 | stored values expecting a previous run (`stored visits_kept = 0` → 2) | the expectation depends on the runner running the file twice | harness |

Triage (judge the form before building it):
- test_print_empty_map:5-7: `{} as text` is "ø", `"a" + {}` is "aø": the empty map printed as ø.
- test_object_blocks:13 `help = {print: "there is help"}; help!` → ø: an object run as code.
- test_unit_tasks:4, 8: `go { 3 m }` awaited is 3, the unit dropped; 3 m (got) looks right.
- test_reflection_words.pairs:22 `dir(m)` of a map is only its keys; lib/prelude.warp dir_of_value gives its keys then its words.
- test_reflection_words.warp:59 `def add(left:int) -> int: left` is refused by decision result-type-as's head rule.
- test_run_time_blocks.pairs:18 `def twice(x): x*2` likewise (`:` after a head names the type).
- test_or_error_type:13 and .warp:7 `as int?` refused "int? is no type".
- test_system_values:26, 27 `mouse_x`, `mouse_down` need a mocked mouse (1207); harness, not language.
- test_test_line_after_tasks:2 expects ø where the runner prints "✓ 1 tests passed".
- test_relational_arms:4 `sorted(xs, >)` (card sorted-comparator-syntax).

## test/ffi/test_js_callbacks.warp natively (card js-array-from-callback, night decision 2026-11-12)
- :9 and :10, a warp function called back by JS that calls JS itself (`Array.from(xs, x => Math.max(x, 2))`), are
  refused natively: the web view's one JS thread waits for the callback's answer. In the page (its own phase) they can work; natively a
  second view or JavaScriptCore in process would (option B on the card, waits for the user).

## Singular field of a list (card item-field-access, parked 2026-11-13)
- test/control/test_call_result_reflection.warp:4, 5 expect `people().name` and `name of people()` to read every
  item's name, ['a', 'b']. Only the plural, `people().names` (:6), reads every item; whether the singular does too
  is a parked decision (it reads two ways: the list's own field or each item's). Left red and untouched until decided.

## eval of a text sees only its with: names (card interpret-data, user 2026-11-13 via warp-interviewer)
- test/node/test_eval_word.pairs.warp:6 `x = 5; eval("x*2")` → 10 is obsolete: eval of a text runs as a program of
  its own and refuses x as unknown; the name goes in by `with:`, `eval("x*2", with: {x: x})` (wiki/evaluation.md,
  wiki/full-evaluation.md). Data and blocks, `interpret(data a+1)`, do see the names where they are written.

## Events and signals, cluster 11 (warp-interviewer 2026-11-13)
Dropped, old Rust-era lines no wiki page backs:
- test/control/test_reactive_ports.pairs.warp `value - was`, `value - before` and test_reactive_ports.warp
  `previous`: `old` is the one word for the value before a write; more names would take ordinary names in every
  listener block.
- test/control/test_on.pairs.warp `y = x--` → 2 with `on set x`: x-- gives the value before the step, 3, as everywhere.
- test/control/test_broadcast.pairs.warp `broadcast {…} on "test-nobody"` and test_named_broadcast.pairs.warp
  `send stop the machine{…} to: "test-nobody"`: an event sent to a channel named by a string is no form; `send … to:`
  is lib/channels' call.
Rewritten to today's values: test_print.pairs.warp:5 (3), test_change_of_expression.pairs.warp:6 (the test summary);
test/modules/test_keys.pairs.warp:2 `key_name(65)` is "a", key names lowercase as the Key enum's cases.
Parked: test_colon_handlers.warp:10 `at 9pm:` on the time library's card.
Key names (warp-interviewer 2026-11-13): test_key_names.warp `key == ArrowDown` dropped (the browser's spelling, the
Key case is down); test/modules/test_keys.pairs.warp:4 `[held_key == ESC, LEFT]` is [no, Key.left], the Key case.

## Error values and try (cluster error-values-try, decided by the user via warp-interviewer 2026-11-13)

| file:line | expected now | proposed | why |
|---|---|---|---|
| test/welcoming/test_welcoming_print.pairs.warp:18, 19 | "a∞" / "aNaN" | Error("divide by zero") | decimal literals are exact: `1.0/0.0` raises; only `x as float` keeps IEEE (wiki/Footguns.md "NaN and infinity") |
| test/numbers/test_nan_observed.pairs.warp:4, 5 | "aNaN" / "NaN" | Error("divide by zero") | same: `-0.0/0.0`, `0.0/0.0` are exact |
| test/welcoming/test_footguns.pairs.warp:321 | true | Error("divide by zero") | same: `1.0/0.0` |
| test/control/test_try_index_and_float.pairs.warp:5 | 7 | refused at compile time | `n = 5; try n#1 else 7`: `#` on a known int can never work, so the compiler refuses, try or not |
| test/control/test_try.pairs.warp:31, 34 | 7 | refused at compile time | same: `def f(m){ m#1 }; try f(3) else 7`, `m.a` of an int |
| test/control/test_error_words.pairs.warp:9 | "ab" | "aøb" | `warn(…)` gives ø, which text shows as ø like any other ø |
| test/welcoming/test_warning_mode.pairs.warp:4 | "ab" | "aøb" | same: `warning(…)` |
| test/control/test_error_not_in_data.pairs.warp:18 | 1 | Error("divide by zero") | `a, b = e, 2` binds a plain variable a, which holds no Error (decision errors held only by optional variables); its own header says the write raises |
