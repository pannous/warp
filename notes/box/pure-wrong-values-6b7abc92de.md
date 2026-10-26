# Pure-lane wrong values, suite 6b7abc92de

Source: /Volumes/Build/measure-pure/suite-6b7abc92de.out on the Mac (5623 failures out of 10720). Extracted failures that are not `got Error(...)`: **515** (500 wrong values, 12 timeouts, 3 crashes on the memory cap). The refusal ranking says 516; the off-by-one is probably one failure its normalizer counted differently. Full machine-readable list with clusters: /workspace/wrong6b7c.json (file, line, src, exp, got, cluster) on the Grok box.

Measured: counts, file:line, and expected vs got. Inferred: the causes, from a short read of the code named in each row.

## Key finding

**196 of the 515 (38%) are a harness artifact, not compiler bugs.** In `src/compiler/pure_tests.rs` `run_child_case`, the check `got == expected` is a strict Node == Node comparison. The Rust `is!` tests these .warp files were translated from (scripts/is_tests.py) compared Node == i64 and Node == char, which are lenient: a Float(2.0) equals 2, and a one-character Text equals a char. Making the harness compare numbers numerically and compare chars symmetrically (`expected == got` for a Char) would turn about 196 tests green. That is a Rust edit of test infrastructure, so it needs Supervisor's OK because Rust is frozen.

## Clusters

| # | cluster | count | scope | cause (inferred) |
|---|---|---|---|---|
| 1 | num_int_vs_float | 103 | harness (no real-program impact) | Got a whole float (2.0 prints as 2) where the expected value is the int 2. The pure harness src/compiler/pure_tests.rs run_child_case compares `got == expected` Node to Node, and Number::eq (src/extensions/numbers.rs) has no Int/Float arm. The Rust is! tests compared Node == i64 (PartialEq<i64>, src/node/comparison.rs:100), which accepts Float(2.0) == 2, so these tests passed there. The pure compiler returns the right value. Fix: compare numerically in the harness (or in Number::eq). |
| 2 | char_vs_text | 93 | harness (no real-program impact) | The expected value is a char 'b' and the program got the one-character text "b". Node::eq is asymmetric: a Char compared with a Text matches, but a Text compared with a Char does not (src/node/comparison.rs: the Text arm has no Char case), and the harness calls got == expected with got as a Text. The Rust is! tests compared Node == char (PartialEq<char>), which accepts a one-character Text. Fix: compare as expected == got, or add a Char case to the Text arm. |
| 3 | other | 68 | mixed | Long tail: overload dispatch (arity, list of int), match alternatives and type arms, `x is (int or text)`, schemas, stack overflow not caught by try, `add "c" to x` on text, `x and= no`, `sort xs!`/`reverse xs!` mutation, `-4|abs`, line continuation `\`, unbounded int negation, `2^62 | 2^61`, {} printed as ø, class copies/aliases, music/draw module globals. |
| 4 | prefix_word_call | 40 | core | A type word (int, text, string, float, double, long, char, type, map) or the host word random spaced before its value is never called and stays a juxtaposed list: [int 3.7]. The rule `($f $x) => $f($x) where $f takes one` in lib/sugar/calls.warp does not fire, because takes-one (value_taking_names in builtin_signatures.warp) leaves out type words and random (random has a zero-argument line), and prefix_calls.warp count_takers also drops type words. Card pure-wrong-type-word-prefix. |
| 5 | exact_real_symbolic | 28 | core (numeric feature gap) | √2 / sqrt(2) gives an f64 where an exact real is expected: √2*√2 == 2, `as text` giving "√2", `is exact`, type real/rational, nearest(rational), and √3^2 == 3. The exact-real number kind is not implemented in the pure lane. |
| 6 | fn_value_not_applied | 24 | core | A function value applied by juxtaposition or through a call on an expression is not called: `sq 3` where sq holds a lambda, `(x => x*2)(5)`, `make_adder(10)(1)`, `(fs#2)(5)`, `g(sq)(5)`. In core/lambdas.warp, lowered_phrase keeps the head word even when it is a local holding a closure, and lowered_call gives a non-symbol head (a call result, a lambda, a picked item) a plain call_node instead of closure_call_<n>. Existing cards nearby: function-value, local-lambda-by-name, function-value-read. |
| 7 | unary_sign_lists | 20 | core (parser/sugar) | `[1 -1]`, `[a -1]` are read as a subtraction, not a list of a signed operand; unary +, `+x`, `++i`, and backtick template interpolation are not handled. |
| 8 | timeout_or_crash | 15 | core (performance) | Lazy ranges over 10^12 (count, #, passed as a parameter) are materialized, which times out or exceeds the memory cap; maps with 100k to 2M keys time out; `while True : 2` runs forever. |
| 9 | type_values | 15 | core | A type value compared with an alias is not equal (integer == int, string == type("ab"), unit/nil/empty == type(ø)); type(1..5) gives "list" not "list of int"; type of a one-character text gives text, not codepoint; type(p) gives class P, not 'P'. |
| 10 | builtin_shadowing | 14 | core | The tests expect builtin and prelude words to win over a user definition of the same name (count(x) := 7; count([1,2]) gives 2; def floor(x) := x+10; floor(2.5) gives 2; round_to and first_items do not shadow the prelude). The pure lane lets the user definition win. |
| 11 | code_data_nodes | 12 | core (quasi-quote / node API) | A data(...) quote is not equal to parse_code(...) or to quasi-quote holes ($a), and `x is data($a * 1)` patterns fail. |
| 12 | for_loop_iteration | 12 | core | `for word in text` iterates characters, not words; `for byte(s) in t` counts characters, not bytes; `for friend in xs` does not filter by class (it should act as a type filter); size in bytes gives no. |
| 13 | type_of_function_signature | 10 | core (reflection) | type(f) gives "class function", not the signature "function: (x) -> int". |
| 14 | logic_rpn_negation | 9 | demo-ish (RPN / old wasm tests) | `true true and` (postfix), `¬ 1`, and `if(2){3}{4}` stay unevaluated phrases. |
| 15 | function_equality | 8 | core (feature) | f == g for structurally or algebraically equal functions is expected to be yes; the pure lane compares identity. |
| 16 | it_lambda_recursion | 8 | core | `f := it ...` recursion (fib := it<2 ...) and `f := it*2; f 3 and f 5` give the wrong value; and/or as a conditional value. |
| 17 | custom_text_printer | 7 | core | A user text(p: P) is not used when a P is added to a text or turned into text. |
| 18 | closure_captured_state | 6 | core | Closures capture by value (decision D7), so counters `n += 1` inside a returned lambda, and `global n` inside a lambda, do not persist. |
| 19 | reactive_binding | 5 | core (feature) | `z := y*y` reactive/charged bindings are evaluated once, eagerly. |
| 20 | list_literal_as_block | 5 | core | `{1 2 3}`, `(1;2;3)`, `1,2,3` give the last item, not a list. |
| 21 | units | 5 | demo-only (units) | `use units; 3m` multiplies by the variable m instead of attaching the unit. |
| 22 | meta_attributes | 4 | core (meta) | @attributes count as keys or values (#p, for k,v in p), and p@unit reads the node. |
| 23 | class_json | 3 | lib (json) | to_json of a class instance wraps it in {"Point":...}. |
| 24 | same_print_other | 1 | harness | Node values print the same, but Node::eq differs (a List with round brackets). |

## Examples (3 per cluster)

### num_int_vs_float (103, harness (no real-program impact))
- `tests/ffi/test_component_short_use.warp:24`: `use m as math\nsqrt(4)` expected `2`, got `2`
- `tests/ffi/test_ffi.pairs.warp:5`: `use math; floor(3.7)` expected `3`, got `3`
- `tests/ffi/test_ffi.warp:4`: `import floor from 'm'\nfloor(3.7)` expected `3`, got `3`

### char_vs_text (93, harness (no real-program impact))
- `tests/control/test_match_else_line.warp:16`: `x = 0\nif x then \"a\"\nelse \"b\"` expected `'b'`, got `"b"`
- `tests/control/test_parsed_code_patterns.warp:19`: `x = data (j: 1\nk: n * 1)\n(x#2).name` expected `'k'`, got `"k"`
- `tests/control/test_parsed_code_patterns.warp:22`: `x = parse(\"j: 1\\nk: n * 1\")\n(x#2).name` expected `'k'`, got `"k"`

### other (68, mixed)
- `tests/control/test_control_words.warp:6`: `f(k:any) := { if k == 1 { return 2.5 } if k == 2 { return 3 } \"sx\" }; f(3)` expected `"sx"`, got `0 0 "sx"`
- `tests/control/test_count_word_blocks.pairs.warp:8`: `n=0; if yes { twice{n+=1} }; n` expected `2`, got `1`
- `tests/control/test_match_alternatives.pairs.warp:11`: `match 1 { 1, 2 => \"small\" }` expected `"small"`, got `1`

### prefix_word_call (40, core)
- `tests/control/test_data_quoting.pairs.warp:4`: `x = data a and b; string x` expected `"a and b"`, got `class string a and b`
- `tests/control/test_data_quoting.pairs.warp:15`: `p = data point{x:1}; string p` expected `"point{x:1}"`, got `class string point{x:1}`
- `tests/control/test_data_quoting.pairs.warp:16`: `m = {a: data point{x:1}}; string m` expected `"{a:point{x:1}}"`, got `class string {a:point{x:1}}`

### exact_real_symbolic (28, core (numeric feature gap))
- `tests/numbers/test_float_exact_context.pairs.warp:12`: `xs = []; xs.add(√2 * √2 == 2); xs#1` expected `yes`, got `no`
- `tests/numbers/test_float_exact_context.pairs.warp:13`: `xs = []; r = √2 * √2; r == 2` expected `yes`, got `no`
- `tests/numbers/test_float_exact_context.pairs.warp:14`: `xs = []; r = √2 * √2; ok = r == 2; xs.add(ok); xs#1` expected `yes`, got `no`

### fn_value_not_applied (24, core)
- `tests/functions/test_alias_definition.pairs.warp:15`: `doubled(x) := x*2; g := &doubled; h := g; h 4` expected `8`, got `doubled 4`
- `tests/functions/test_anonymous_application.pairs.warp:5`: `(x => x*2) 4` expected `8`, got `closure_lambda_1 4`
- `tests/functions/test_anonymous_function_forms.pairs.warp:17`: `(() => 7)()` expected `7`, got `closure_lambda_1 ø`

### unary_sign_lists (20, core (parser/sugar))
- `tests/functions/test_builtin_clash.pairs.warp:16`: `round(x) := x+10; round(2.4) * 2` expected `4`, got `24.8`
- `tests/lists/test_bare_list_assignment.pairs.warp:10`: `{1+1 2+2}` expected `[2, 4]`, got `4`
- `tests/lists/test_list_literal_sign.pairs.warp:4`: `a=5; [a -1]` expected `[5, -1]`, got `[4]`

### timeout_or_crash (15, core (performance))
- `tests/control/test_for_forms.warp:7`: `s=\"\"; for line in \"a\nb c\" { s += line + \",\" }; s` expected `"a,b c,"`, got `crashed (exit status: 1):`
- `tests/lists/test_down_and_letter_ranges.pairs.warp:8`: `letters = 'a' to 'e'; #letters` expected `5`, got `crashed (exit status: 1): memory cap: growing a me`
- `tests/lists/test_lazy_range.pairs.warp:10`: `xs = 1..1000000000000; xs.count` expected `999999999999`, got `timed out after 30 s`

### type_values (15, core)
- `tests/lists/test_range_values.pairs.warp:7`: `type(1..5)` expected `"list of int"`, got `class list`
- `tests/numbers/test_long_decimals.pairs.warp:6`: `type(1.0000000000000000001)` expected `"rational"`, got `class int`
- `tests/text/test_char_parameter_type.pairs.warp:4`: `def g(t) := type(t); g(\"a\")` expected `"codepoint"`, got `class text`

### builtin_shadowing (14, core)
- `tests/functions/test_builtin_clash.pairs.warp:5`: `count(x) := 7; count([1,2])` expected `2`, got `7`
- `tests/functions/test_builtin_clash.pairs.warp:14`: `def floor(x) := x+10; floor(2.5)` expected `2`, got `12.5`
- `tests/functions/test_builtin_clash.pairs.warp:15`: `def ceil(x) := x+10; ceil 2.5` expected `3`, got `12.5`

### code_data_nodes (12, core (quasi-quote / node API))
- `tests/control/test_code_holes.warp:44`: `x = data (n * 1)\nx is data($a * 1)` expected `yes`, got `no`
- `tests/control/test_data_quoting.pairs.warp:12`: `string(data point{x:1})` expected `"point{x:1}"`, got `"point:{x:1}"`
- `tests/control/test_quasi_quote.pairs.warp:5`: `data(a of b) == parse_code(\"a of b\")` expected `yes`, got `no`

### for_loop_iteration (12, core)
- `tests/control/test_filter_loops.warp:4`: `class friend{n:int}; class foe{n:int}; xs=[friend(1), foe(2), friend(3)]; s=0; for friend ` expected `4`, got `6`
- `tests/control/test_filter_loops.warp:5`: `class friend{n:int}; class foe{n:int}; xs=[friend(1), foe(2), friend(3)]; c=0; for friend ` expected `2`, got `3`
- `tests/control/test_for_forms.pairs.warp:24`: `text=\"hi  you\"; s=\"\"; for word in text { s += word + \",\" }; s` expected `"hi,you,"`, got `"h,i, , ,y,o,u,"`

### type_of_function_signature (10, core (reflection))
- `tests/control/test_function_dir.warp:12`: `area(width: int, height: int) := width * height\ntype(area)` expected `"function: (width: int, height: int) -> `, got `class function`
- `tests/control/test_function_dir.warp:14`: `area(width: int, height: int) := width * height\ntype(&area)` expected `"function: (width: int, height: int) -> `, got `class function`
- `tests/control/test_function_objects.pairs.warp:4`: `type(sqrt)` expected `"builtin: (x:number) -> number"`, got `class float`

### logic_rpn_negation (9, demo-ish (RPN / old wasm tests))
- `tests/parser/test_surface_syntax.pairs.warp:17`: `¬ false` expected `yes`, got `no no`
- `tests/wasm/test_wasm.pairs.warp:252`: `true true and` expected `yes`, got `yes ø`
- `tests/wasm/test_wasm.pairs.warp:253`: `false true and` expected `no`, got `no ø`

### function_equality (8, core (feature))
- `tests/functions/test_runtime_function_equality.pairs.warp:7`: `xs = [x => x + 1, y => y + 1]; xs#1 == xs#2` expected `yes`, got `no`
- `tests/functions/test_runtime_function_equality.pairs.warp:8`: `f(x) := x + 1; g(y) := y + 1; xs = [&f, &g]; xs#1 == xs#2` expected `yes`, got `no`
- `tests/welcoming/test_footguns.pairs.warp:436`: `f(x):=x+1; g(y):=y+1; f==g` expected `yes`, got `no`

### it_lambda_recursion (8, core)
- `tests/operators/test_mutating_bang.pairs.warp:14`: `twice:=it*2; x=3; twice x!; x` expected `6`, got `3`
- `tests/operators/test_pipeline.warp:12`: `square := it*it; numbers=[1 2 3 4]; square numbers \|> filter(x => x > 5) \|> sum` expected `25`, got `0`
- `tests/parser/test_newline_precedence.warp:18`: `fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)` expected `56`, got `55`

### custom_text_printer (7, core)
- `tests/text/test_text_plus_printable.warp:4`: `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\n\"hi \" + bo` expected `"hi person Bo"`, got `"hi P{name:\"Bo\"}"`
- `tests/text/test_text_plus_printable.warp:8`: `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\n\"hi \" + bo + \"!` expected `"hi person Bo!"`, got `"hi P{name:\"Bo\"}!"`
- `tests/text/test_text_plus_printable.warp:12`: `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\nbo + \" there\"` expected `"person Bo there"`, got `"P{name:\"Bo\"} there"`

### closure_captured_state (6, core)
- `tests/functions/test_lambda_global.pairs.warp:4`: `n = 0; f = () => { global n; n += 1 }; f(); f(); n` expected `2`, got `0`
- `tests/functions/test_lambda_global.pairs.warp:5`: `global n = 0; f = () => { global n; n += 1 }; f(); f(); f(); n` expected `3`, got `0`
- `tests/functions/test_nonlocal_lambdas.pairs.warp:4`: `def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c()` expected `2`, got `1`

### reactive_binding (5, core (feature))
- `tests/control/test_reactive_ports.pairs.warp:4`: `count = 0; doubled := count * 2; count = 3; doubled` expected `6`, got `0`
- `tests/control/test_reactive_ports.warp:5`: `first = \"Ada\"; last = \"L\"; full := first + \" \" + last; first = \"Grace\"; full` expected `"Grace L"`, got `"Ada L"`
- `tests/functions/test_charged_getters.pairs.warp:4`: `y=3; z:=y*y; y=4; z` expected `16`, got `9`

### list_literal_as_block (5, core)
- `tests/lists/test_lists.pairs.warp:61`: `(1;2;3)` expected `[1, 2, 3]`, got `3`
- `tests/lists/test_lists.pairs.warp:62`: `1,2,3` expected `[1, 2, 3]`, got `3`
- `tests/lists/test_lists.pairs.warp:66`: `{1 2 3}` expected `[1, 2, 3]`, got `3`

### units (5, demo-only (units))
- `tests/numbers/test_unit_beside_variable.pairs.warp:4`: `use units; m = 5; str(3m)` expected `"3m"`, got `"15"`
- `tests/numbers/test_unit_beside_variable.warp:4`: `use units\ns = 2\nstr(0.5s)` expected `"0.5s"`, got `"1"`
- `tests/numbers/test_units_after_numbers.warp:4`: `use units\ns = 3\nstr(0.5s)` expected `"0.5s"`, got `"1.5"`

### meta_attributes (4, core (meta))
- `tests/lists/test_map_starts.pairs.warp:9`: `p = {x:1}; p.@source = \"gps\"; #p` expected `1`, got `2`
- `tests/parser/test_meta_attributes.pairs.warp:13`: `p = {x:1 @unit:\"cm\"}; p@unit` expected `"cm"`, got `{x:1}`
- `tests/parser/test_meta_attributes.pairs.warp:23`: `p = {x:1}; p.@s = 30; t=0; for k,v in p {t+=v}; t` expected `1`, got `31`

### class_json (3, lib (json))
- `tests/types/test_class_json.pairs.warp:4`: `use json; class Point{x:int y:int}; to_json(Point(1, 2))` expected `"{\"x\":1,\"y\":2}"`, got `"{\"Point\":{\"x\":1,\"y\":2}}"`
- `tests/types/test_class_json.warp:4`: `use json; class Point{x:int y:int}; to_json([Point(1, 2), Point(3, 4)])` expected `"[{\"x\":1,\"y\":2},{\"x\":3,\"y\":4}]"`, got `"[{\"Point\":{\"x\":1,\"y\":2}},{\"Point\":{\"x\":`
- `tests/types/test_class_json.warp:5`: `use json; class Point{x:int y:int}; class Line{a:Point b:Point}; to_json(Line(Point(1, 2),` expected `"{\"a\":{\"x\":1,\"y\":2},\"b\":{\"x\":3`, got `"{\"Line\":{\"a\":{\"Point\":{\"x\":1,\"y\":2}},\"`

### same_print_other (1, harness)
- `tests/node/test_node_cases.pairs.warp:7`: `List([1, 2], Bracket.Round, Separator.Comma)` expected `(1, 2)`, got `(1, 2)`

## Card

Biggest **core** cluster: prefix_word_call (40). I added the card `pure-wrong-type-word-prefix` in Soon, unclaimed (todo filed it under topic parsing-2). Before adding it I checked `todo list --json`: none of the related cards cover this case. pure-type-values is about a type word used as a value being refused, and spaced-library (Done) moved `reverse xs` to the lib/sugar/calls.warp rule whose `takes one` condition leaves out these words.

Suggested owner: **warp-layers**, which owns the sugar cards. The rule lives in lib/sugar/calls.warp (`($f $x) => $f($x) where $f takes one`), and the fix is either widening `takes one` to type words and random or adding a type-word rule. If the fix lands in the `takes one` where-condition in rewrites.warp, Warp Code owns that part.

Next core cluster: fn_value_not_applied (24), in core/lambdas.warp `lowered_phrase` and `lowered_call`. It has no owner on the list, so it would fall to grok-warp-code-2 (us).

## All 515 tests

| file:line | src | expected | got | cluster |
|---|---|---|---|---|
| tests/control/test_code_holes.warp:44 | `x = data (n * 1)\nx is data($a * 1)` | `yes` | `no` | code_data_nodes |
| tests/control/test_control_words.warp:6 | `f(k:any) := { if k == 1 { return 2.5 } if k == 2 { return 3 } \"sx\" }; f(3)` | `"sx"` | `0 0 "sx"` | other |
| tests/control/test_count_word_blocks.pairs.warp:8 | `n=0; if yes { twice{n+=1} }; n` | `2` | `1` | other |
| tests/control/test_data_quoting.pairs.warp:4 | `x = data a and b; string x` | `"a and b"` | `class string a and b` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:12 | `string(data point{x:1})` | `"point{x:1}"` | `"point:{x:1}"` | code_data_nodes |
| tests/control/test_data_quoting.pairs.warp:15 | `p = data point{x:1}; string p` | `"point{x:1}"` | `class string point{x:1}` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:16 | `m = {a: data point{x:1}}; string m` | `"{a:point{x:1}}"` | `class string {a:point{x:1}}` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:17 | `m = {a:{b:1}}; string m` | `"{a:{b:1}}"` | `class string {a{b:1}}` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:18 | `e = data a:1; string e` | `"{a:1}"` | `class string a:1` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:21 | `string [data a or b, 3]` | `"[a or b 3]"` | `class string [a or b 3]` | prefix_word_call |
| tests/control/test_data_quoting.pairs.warp:22 | `xs = [data a or b, 3]; string xs` | `"[a or b 3]"` | `class string [a or b 3]` | prefix_word_call |
| tests/control/test_filter_loops.warp:4 | `class friend{n:int}; class foe{n:int}; xs=[friend(1), foe(2), friend(3)]; s=0; for friend in xs { s += friend.n }; s` | `4` | `6` | for_loop_iteration |
| tests/control/test_filter_loops.warp:5 | `class friend{n:int}; class foe{n:int}; xs=[friend(1), foe(2), friend(3)]; c=0; for friend in xs: c += 1\nc` | `2` | `3` | for_loop_iteration |
| tests/control/test_for_forms.pairs.warp:24 | `text=\"hi  you\"; s=\"\"; for word in text { s += word + \",\" }; s` | `"hi,you,"` | `"h,i, , ,y,o,u,"` | for_loop_iteration |
| tests/control/test_for_forms.warp:7 | `s=\"\"; for line in \"a\nb c\" { s += line + \",\" }; s` | `"a,b c,"` | `crashed (exit status: 1):` | timeout_or_crash |
| tests/control/test_for_unit_words.pairs.warp:5 | `t = \"héllo\"; n = 0; for bytes in t: n += 1; n` | `6` | `5` | for_loop_iteration |
| tests/control/test_for_unit_words.pairs.warp:12 | `t = \"héllo\"; n = 0; for byte in t: n += 1; n` | `6` | `5` | for_loop_iteration |
| tests/control/test_function_dir.warp:12 | `area(width: int, height: int) := width * height\ntype(area)` | `"function: (width: int, height: int) -> int"` | `class function` | type_of_function_signature |
| tests/control/test_function_dir.warp:14 | `area(width: int, height: int) := width * height\ntype(&area)` | `"function: (width: int, height: int) -> int"` | `class function` | type_of_function_signature |
| tests/control/test_function_objects.pairs.warp:4 | `type(sqrt)` | `"builtin: (x:number) -> number"` | `class float` | type_of_function_signature |
| tests/control/test_function_objects.warp:70 | `use math\ntype(gcd)` | `"function: (a: int, b: int) -> int"` | `class function` | type_of_function_signature |
| tests/control/test_function_value_words.warp:6 | `sq(x) := x * x\ntype(sq)` | `"function: (x) -> int"` | `class function` | type_of_function_signature |
| tests/control/test_library_signatures.pairs.warp:32 | `f(b) := b == 0; type(function f)` | `"function: (b) -> bool"` | `class function` | type_of_function_signature |
| tests/control/test_match_alternatives.pairs.warp:11 | `match 1 { 1, 2 => \"small\" }` | `"small"` | `1` | other |
| tests/control/test_match_else_line.warp:16 | `x = 0\nif x then \"a\"\nelse \"b\"` | `'b'` | `"b"` | char_vs_text |
| tests/control/test_match_type_arms.pairs.warp:9 | `x = 3; x is (int or text)` | `yes` | `no` | other |
| tests/control/test_match_type_arms.pairs.warp:13 | `text = 3; match 3 { text => \"three\"; _ => \"other\" }` | `"three"` | `"other"` | other |
| tests/control/test_match_type_arms.pairs.warp:16 | `x = 2.5; x is int or text` | `no` | `class text` | other |
| tests/control/test_method_let_shadows_field.warp:28 | `class F { name: text; body: any }\nfresh(f: F) := {\n\tbody = 99\n\tbody\n}\nf = F(\"main\", 42)\nfresh(f)\nf.body` | `99` | `42` | other |
| tests/control/test_parsed_code_patterns.warp:19 | `x = data (j: 1\nk: n * 1)\n(x#2).name` | `'k'` | `"k"` | char_vs_text |
| tests/control/test_parsed_code_patterns.warp:22 | `x = parse(\"j: 1\\nk: n * 1\")\n(x#2).name` | `'k'` | `"k"` | char_vs_text |
| tests/control/test_print_empty_map.pairs.warp:5 | `{} as text` | `'ø'` | `"{}"` | other |
| tests/control/test_print_empty_map.pairs.warp:6 | `\"a\" + {}` | `"aø"` | `"a{}"` | other |
| tests/control/test_print_empty_map.pairs.warp:7 | `x = {}; \"a\" + x` | `"aø"` | `"a{}"` | other |
| tests/control/test_quasi_quote.pairs.warp:5 | `data(a of b) == parse_code(\"a of b\")` | `yes` | `no` | code_data_nodes |
| tests/control/test_quasi_quote.warp:28 | `a = data(y)\ndata($a + 1) == data(y + 1)` | `yes` | `no` | code_data_nodes |
| tests/control/test_quasi_quote.warp:30 | `a = data(y * 2)\ndata(f($a, 3)) == data(f(y * 2, 3))` | `yes` | `no` | code_data_nodes |
| tests/control/test_quasi_quote.warp:32 | `n = data(k)\ndata([$n, 2]) == data([k, 2])` | `yes` | `no` | code_data_nodes |
| tests/control/test_quasi_quote.warp:36 | `k = data(width)\ndata(m.pop($(str(k)))) == data(m.pop(\"width\"))` | `yes` | `no` | code_data_nodes |
| tests/control/test_quoted_data_untouched.pairs.warp:4 | `data (a.warp) == parse_code(\"a.warp\")` | `yes` | `(a.warp)==(parse_code "a.warp")` | code_data_nodes |
| tests/control/test_quoted_data_untouched.pairs.warp:5 | `x = data(a.b.c); x.op` | `'.'` | `"."` | char_vs_text |
| tests/control/test_quoted_data_untouched.pairs.warp:10 | `data(x is int) == parse_code(\"x is int\")` | `yes` | `no` | code_data_nodes |
| tests/control/test_reactive_ports.pairs.warp:4 | `count = 0; doubled := count * 2; count = 3; doubled` | `6` | `0` | reactive_binding |
| tests/control/test_reactive_ports.warp:5 | `first = \"Ada\"; last = \"L\"; full := first + \" \" + last; first = \"Grace\"; full` | `"Grace L"` | `"Ada L"` | reactive_binding |
| tests/control/test_schemas.pairs.warp:4 | `{port: 80, host: \"x\"} is {port: int, host: text}` | `yes` | `no` | other |
| tests/control/test_text_loop_value.pairs.warp:10 | `for c in \"ab\" { c }` | `'b'` | `"b"` | char_vs_text |
| tests/control/test_text_loop_value.pairs.warp:11 | `for c in \"ab\" { if c == \"b\" { break }; c }` | `'a'` | `"a"` | char_vs_text |
| tests/control/test_text_loop_value.pairs.warp:20 | `for n in [\"a\" \"b\"] { if n == \"b\" { continue }; n }` | `'a'` | `"a"` | char_vs_text |
| tests/control/test_try_stack_overflow.warp:4 | `down(n) := if n == 0 then 0 else 1 + down(n - 1); try down(1000000) else 7` | `7` | `1000000` | other |
| tests/control/test_try_stack_overflow.warp:7 | `down(n) := if n == 0 then 0 else 1 + down(n - 1); r = try down(1000000) else 100; r + down(5)` | `105` | `1000005` | other |
| tests/control/test_type_word_filter_loops.pairs.warp:15 | `xs=[1, \"a\"]; s=\"\"; y=xs#2; if 1 { s += y }; s` | `'a'` | `"a"` | char_vs_text |
| tests/control/test_unwrap.pairs.warp:6 | `x=\"a\"; x!` | `'a'` | `"a"` | char_vs_text |
| tests/control/test_when_signals.pairs.warp:11 | `x = 3; match x { 1 -> \"a\"; else -> \"b\" }` | `'b'` | `"b"` | char_vs_text |
| tests/ffi/test_component_short_use.warp:24 | `use m as math\nsqrt(4)` | `2` | `2` | num_int_vs_float |
| tests/ffi/test_ffi.pairs.warp:5 | `use math; floor(3.7)` | `3` | `3` | num_int_vs_float |
| tests/ffi/test_ffi.warp:4 | `import floor from 'm'\nfloor(3.7)` | `3` | `3` | num_int_vs_float |
| tests/ffi/test_ffi_one_parser.pairs.warp:4 | `sqrt(16)` | `4` | `4` | num_int_vs_float |
| tests/functions/test_alias_definition.pairs.warp:15 | `doubled(x) := x*2; g := &doubled; h := g; h 4` | `8` | `doubled 4` | fn_value_not_applied |
| tests/functions/test_anonymous_application.pairs.warp:5 | `(x => x*2) 4` | `8` | `closure_lambda_1 4` | fn_value_not_applied |
| tests/functions/test_anonymous_function_forms.pairs.warp:17 | `(() => 7)()` | `7` | `closure_lambda_1 ø` | fn_value_not_applied |
| tests/functions/test_arity_and_type_overloads.warp:4 | `class Box{v}\nf(x) := 1\nf(x, y) := f(x) + 1\nf(b: Box) := 3\nf(1, 2) * 10 + f(Box(5))` | `23` | `21` | other |
| tests/functions/test_arity_overloads.warp:4 | `def log(x) { log(x, 10) }; def log(x, base) { x/base }; log(50)` | `3.912023005428146` | `5` | other |
| tests/functions/test_bare_function_name.pairs.warp:7 | `f(x=1):=x*2;f` | `2` | `f` | other |
| tests/functions/test_bare_host_words.pairs.warp:14 | `type random` | `"float"` | `class type 0.28418991406327965` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:32 | `any([random 1...4 for i in 1 to 80], x => x == 4)` | `yes` | `no` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:33 | `type [random int for i in 1 to 2]` | `"list of int"` | `class type [ 0.7145982947724333 class int   0.43170667949968466 class int ]` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:34 | `random 1..2` | `1` | `0.41646813110842873 [1]` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:35 | `random 3...3` | `3` | `0.8304140200511884 [3]` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:36 | `random 4 to 4` | `4` | `0.716306888447707 [4]` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:48 | `use prelude; random_until(1, 2)` | `1` | `1` | num_int_vs_float |
| tests/functions/test_bare_host_words.pairs.warp:49 | `use prelude; random_between(3, 3)` | `3` | `3` | num_int_vs_float |
| tests/functions/test_bare_host_words.pairs.warp:51 | `use prelude; type random_int()` | `"int"` | `class type 3960486524627725516` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:52 | `use prelude; type random_bool()` | `"bool"` | `class type yes` | prefix_word_call |
| tests/functions/test_bare_host_words.pairs.warp:53 | `use prelude; type random_codepoint()` | `"codepoint"` | `class type "󉡪"` | prefix_word_call |
| tests/functions/test_braceless_closure_call.pairs.warp:5 | `add(a, b) := a + b; g = function add; g 1 2` | `3` | `add 1 2` | fn_value_not_applied |
| tests/functions/test_braceless_closure_call.pairs.warp:6 | `f = x => x + 1; f 3 == 4` | `yes` | `closure_lambda_1 no` | fn_value_not_applied |
| tests/functions/test_builtin_clash.pairs.warp:5 | `count(x) := 7; count([1,2])` | `2` | `7` | builtin_shadowing |
| tests/functions/test_builtin_clash.pairs.warp:14 | `def floor(x) := x+10; floor(2.5)` | `2` | `12.5` | builtin_shadowing |
| tests/functions/test_builtin_clash.pairs.warp:15 | `def ceil(x) := x+10; ceil 2.5` | `3` | `12.5` | builtin_shadowing |
| tests/functions/test_builtin_clash.pairs.warp:16 | `round(x) := x+10; round(2.4) * 2` | `4` | `24.8` | unary_sign_lists |
| tests/functions/test_builtin_clash.pairs.warp:17 | `def floor(x) := x+10; floor(2)` | `2` | `12` | builtin_shadowing |
| tests/functions/test_captured_containers.pairs.warp:6 | `k = {a: 1.5}; g := it*k.a; g(2)` | `3` | `3` | num_int_vs_float |
| tests/functions/test_captured_containers.pairs.warp:11 | `k = [1.5, 2.5]; g(x) := x*k#1; g(2)` | `3` | `3` | num_int_vs_float |
| tests/functions/test_captured_node_return.pairs.warp:4 | `use list; variance([float(1), float(3)])` | `1` | `1` | num_int_vs_float |
| tests/functions/test_character_results.pairs.warp:4 | `def f(){ 'b' }; f()` | `'b'` | `"b"` | char_vs_text |
| tests/functions/test_character_results.pairs.warp:5 | `def f(x){ x }; f('b')` | `'b'` | `"b"` | char_vs_text |
| tests/functions/test_character_results.pairs.warp:6 | `def f(x){ x }; f(\"b\")` | `'b'` | `"b"` | char_vs_text |
| tests/functions/test_character_results.pairs.warp:7 | `def f(x:char){ x }; f('b')` | `'b'` | `"b"` | char_vs_text |
| tests/functions/test_charged_getters.pairs.warp:4 | `y=3; z:=y*y; y=4; z` | `16` | `9` | reactive_binding |
| tests/functions/test_charged_getters.pairs.warp:5 | `y=3; z:=y*y; a=z; y=4; a+z` | `25` | `18` | reactive_binding |
| tests/functions/test_charged_getters.warp:4 | `y=3\nz:=y*y\ny=4\nz` | `16` | `9` | reactive_binding |
| tests/functions/test_closures.pairs.warp:8 | `make_adder(n) := x => x+n; make_adder(10)(1)` | `11` | `closure_lambda_1 1` | fn_value_not_applied |
| tests/functions/test_closures.pairs.warp:25 | `scale(k) := (x => x*k); s = scale(1.5); s(2)` | `3` | `3` | num_int_vs_float |
| tests/functions/test_closures.pairs.warp:31 | `add(a) := (b => (c => a+b+c)); add(1)(2)(3)` | `6` | `closure_lambda_1 2 3` | fn_value_not_applied |
| tests/functions/test_closures.pairs.warp:45 | `fs=[x=>x*2, x=>x+1]; (fs#2)(5)` | `6` | `closure_lambda_2 5` | fn_value_not_applied |
| tests/functions/test_closures.pairs.warp:67 | `add(a) := b => a + b; xs = []; xs.push(add(2)(5)); xs#1` | `7` | `closure_lambda_1` | fn_value_not_applied |
| tests/functions/test_function_typed_parameters.warp:7 | `apply(f: (float) -> float, x: float) := f(x)\nwhole(t) := 3\napply(whole, 1.0) is float` | `yes` | `no` | other |
| tests/functions/test_function_typed_parameters.warp:12 | `apply(f: (float) -> float, x: float) := f(x)\nouter(g, x) := apply(g, x)\nwhole(t) := 3\nouter(whole, 1.0) is float` | `yes` | `no` | other |
| tests/functions/test_function_values.pairs.warp:12 | `double2(x):=x*2; g=function double2; g 4` | `8` | `double2 4` | fn_value_not_applied |
| tests/functions/test_function_words_consume.pairs.warp:11 | `sqrt 4 + 5` | `3` | `3` | num_int_vs_float |
| tests/functions/test_lambda_global.pairs.warp:4 | `n = 0; f = () => { global n; n += 1 }; f(); f(); n` | `2` | `0` | closure_captured_state |
| tests/functions/test_lambda_global.pairs.warp:5 | `global n = 0; f = () => { global n; n += 1 }; f(); f(); f(); n` | `3` | `0` | closure_captured_state |
| tests/functions/test_lambda_held_by_field.warp:4 | `h = {d: x => x * 3}\n(h.d)(5)` | `15` | `closure_lambda_1 5` | fn_value_not_applied |
| tests/functions/test_lambda_spaced_body.pairs.warp:14 | `(x => x * 2)(5)` | `10` | `closure_lambda_1 5` | fn_value_not_applied |
| tests/functions/test_lambda_tuple_arguments.pairs.warp:9 | `(x -> x * 2)(7)` | `14` | `closure_lambda_1 7` | fn_value_not_applied |
| tests/functions/test_lambdas.pairs.warp:4 | `sq = x=>x*x; sq 3` | `9` | `closure_lambda_1 3` | fn_value_not_applied |
| tests/functions/test_lambdas.pairs.warp:6 | `add = (x y)->x+y; add 2 3` | `5` | `closure_lambda_1 2 3` | fn_value_not_applied |
| tests/functions/test_lambdas.pairs.warp:11 | `n=10; f = x=>x+n; f 1` | `11` | `closure_lambda_1 1` | fn_value_not_applied |
| tests/functions/test_map_library_word_first.pairs.warp:12 | `floor(x) := x + 1; map floor [1]` | `[2]` | `class map floor [1]` | prefix_word_call |
| tests/functions/test_member_phrases.pairs.warp:13 | `\"äb\".size in bytes` | `3` | `no` | for_loop_iteration |
| tests/functions/test_method_syntax_calls.pairs.warp:5 | `x=2.4; x.round()` | `2` | `2` | num_int_vs_float |
| tests/functions/test_named_arguments.warp:9 | `global i = 0; g() := { i = i * 10 + 1; i }; h() := { i = i * 10 + 2; i }; f(a, b) := a * 100 + b; f(b=h(), a=g())` | `2102` | `112` | other |
| tests/functions/test_named_arguments.warp:11 | `global i = 0; g() := { i = i * 10 + 1; i }; h() := { i = i * 10 + 2; i }; f(a, b) := a * 100 + b; k() := f(b=h(), a=g())` | `2102` | `112` | other |
| tests/functions/test_nonlocal_lambdas.pairs.warp:4 | `def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c()` | `2` | `1` | closure_captured_state |
| tests/functions/test_nonlocal_lambdas.pairs.warp:5 | `def acc(){ total=0; x=>{ total+=x; total } }; a=acc(); a(5); a(10)` | `15` | `10` | closure_captured_state |
| tests/functions/test_nonlocal_lambdas.warp:4 | `def counter(){ n=0; return () => { n = n + 1; return n } }; c=counter(); c(); c(); c()` | `3` | `1` | closure_captured_state |
| tests/functions/test_ocaml_fun.pairs.warp:4 | `map (x -> x * 2) [1, 2, 3]` | `[2 4 6]` | `class map closure_lambda_1 [1 2 3]` | fn_value_not_applied |
| tests/functions/test_parameter_map_patterns.warp:4 | `greet({name}) := \"hi \" + name\ngreet({name: \"Ann\"})` | `"hi Ann"` | `"hi {name:\"Ann\"}"` | other |
| tests/functions/test_parameter_overloads.warp:12 | `combine(a:float, b:float) := a+b; combine(a:int, b:int) := a*b; def g(p){ combine(p, p) }; g(2)` | `4` | `4` | num_int_vs_float |
| tests/functions/test_parameter_overloads.warp:13 | `combine(a:float, b:float) := a+b; combine(a:int, b:int) := a*b; def g(p){ combine(p, p) }; g(1.5)` | `3` | `3` | num_int_vs_float |
| tests/functions/test_parameter_overloads.warp:27 | `play(xs: list of int) := 1; play(xs: list) := 2; play([1]) * 10 + play([\"a\"])` | `12` | `22` | other |
| tests/functions/test_passed_through_functions.pairs.warp:4 | `g = x => x; g(y => y*2)(4)` | `8` | `closure_lambda_2 4` | fn_value_not_applied |
| tests/functions/test_passed_through_functions.pairs.warp:5 | `sq(y) := y*y; g = x => x; g(sq)(5)` | `25` | `sq 5` | fn_value_not_applied |
| tests/functions/test_picked_calls.pairs.warp:4 | `fs = [x=>x+1, x=>x*2]; fs[1](3)` | `6` | `closure_lambda_2 3` | fn_value_not_applied |
| tests/functions/test_property_access.pairs.warp:14 | `p={name:\"Joe\"}; p's name` | `"Joe"` | `{name:"Joe"} "s name"` | other |
| tests/functions/test_python_ruby_definitions.warp:76 | `n = 0\ninc = () => { n += 1; n }\ninc()\ninc()` | `2` | `1` | closure_captured_state |
| tests/functions/test_random_dispatch.pairs.warp:4 | `random [7,7]` | `7` | `0.29812141028056305 [7 7]` | prefix_word_call |
| tests/functions/test_random_dispatch.pairs.warp:5 | `xs=[7,7]; random xs` | `7` | `0.902078590394706 [7 7]` | prefix_word_call |
| tests/functions/test_random_dispatch.pairs.warp:21 | `random 6 < 6` | `yes` | `0.17249189463025572 no` | prefix_word_call |
| tests/functions/test_random_dispatch.pairs.warp:24 | `random 2.5 < 2.5 and yes` | `yes` | `0.10054977779359353 no` | prefix_word_call |
| tests/functions/test_return_type_dispatch.pairs.warp:4 | `class pdf{body}; make(t):pdf := pdf(t); m = make(\"a\"); m.body` | `'a'` | `"a"` | char_vs_text |
| tests/functions/test_return_type_dispatch.pairs.warp:5 | `class pdf{body}; make(t) := pdf(t); m = make(\"a\"); m.body` | `'a'` | `"a"` | char_vs_text |
| tests/functions/test_round8_call_forms.pairs.warp:4 | `[1, 2, 3].sum { \|x\| x * 2 }` | `12` | `6 closure_lambda_1` | fn_value_not_applied |
| tests/functions/test_round8_call_forms.pairs.warp:5 | `[1, 2, 3].count { \|x\| x > 1 }` | `2` | `3 closure_lambda_1` | fn_value_not_applied |
| tests/functions/test_runtime_function_equality.pairs.warp:7 | `xs = [x => x + 1, y => y + 1]; xs#1 == xs#2` | `yes` | `no` | function_equality |
| tests/functions/test_runtime_function_equality.pairs.warp:8 | `f(x) := x + 1; g(y) := y + 1; xs = [&f, &g]; xs#1 == xs#2` | `yes` | `no` | function_equality |
| tests/functions/test_sequence_values.warp:4 | `f(x) := {\n1\nx\n}; f(\"a\")` | `'a'` | `"a"` | char_vs_text |
| tests/functions/test_sequence_values.warp:16 | `f = (x) => {\n1\nx\n}; f(\"a\")` | `'a'` | `"a"` | char_vs_text |
| tests/functions/test_undefined_calls.pairs.warp:5 | `sqrt(4)` | `2` | `2` | num_int_vs_float |
| tests/functions/test_whole_float_to_int_parameter.pairs.warp:5 | `f(x:float) := x+1; f(2.0)` | `3` | `3` | num_int_vs_float |
| tests/lists/test_bare_list_assignment.pairs.warp:10 | `{1+1 2+2}` | `[2, 4]` | `4` | unary_sign_lists |
| tests/lists/test_concat_keeps_arrays.pairs.warp:8 | `xs = [1.5 2.5]; ys = []; ys.add(0.5); c = ys + xs; c[1] + c[2]` | `4` | `4` | num_int_vs_float |
| tests/lists/test_down_and_letter_ranges.pairs.warp:8 | `letters = 'a' to 'e'; #letters` | `5` | `crashed (exit status: 1): memory cap: growing a memory to 4096 MB passes the cap of 2048 MB (WARP_MEMORY_CAP_MB)` | timeout_or_crash |
| tests/lists/test_element_wise_lists.pairs.warp:26 | `xs = float[3]; xs#1 = 0.5; s = 7; s = 1.5 + xs#1; s` | `2` | `2` | num_int_vs_float |
| tests/lists/test_element_wise_vectors.pairs.warp:23 | `use math; norm([3, 4])` | `5` | `5` | num_int_vs_float |
| tests/lists/test_element_wise_vectors.pairs.warp:24 | `use math; v = [3, -4]; norm v` | `5` | `5` | num_int_vs_float |
| tests/lists/test_element_wise_vectors.pairs.warp:25 | `use math; magnitude([6, 8])` | `10` | `10` | num_int_vs_float |
| tests/lists/test_float_array_writes.pairs.warp:4 | `xs = float[3]; xs#2 = 2.5; xs#3 = 4; xs` | `[0, 2.5, 4]` | `[0 2.5 4]` | num_int_vs_float |
| tests/lists/test_float_array_writes.pairs.warp:5 | `xs = float[100000]; for i in 1 to 100000 { xs#i = i }; xs#90000` | `90000` | `90000` | num_int_vs_float |
| tests/lists/test_float_array_writes.pairs.warp:6 | `xs = float[100000]; for i in 1 to 100000 { xs#i = i * 1.5 }; xs#2` | `3` | `3` | num_int_vs_float |
| tests/lists/test_float_array_writes.warp:4 | `xs = float[1000]; for i in 1 to 1000 { xs#i = i }; s = 0.0; for x in xs { s += x }; s` | `500500` | `500500` | num_int_vs_float |
| tests/lists/test_global_typed_lists.pairs.warp:29 | `out = [\"a\"]; add(t) := { global out; out += [t] }; add(\"b\"); out#2` | `'b'` | `"b"` | char_vs_text |
| tests/lists/test_lazy_range.pairs.warp:10 | `xs = 1..1000000000000; xs.count` | `999999999999` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_lazy_range.pairs.warp:26 | `count 1..10^12` | `999999999999` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_lazy_range.pairs.warp:27 | `r = 1..10^12; r#7` | `7` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_list_literal_sign.pairs.warp:4 | `a=5; [a -1]` | `[5, -1]` | `[4]` | unary_sign_lists |
| tests/lists/test_list_literal_sign.pairs.warp:5 | `a=5; [a +1]` | `[5, 1]` | `[6]` | unary_sign_lists |
| tests/lists/test_list_literal_sign.pairs.warp:6 | `a=5; [a*2 -1 a]` | `[10, -1, 5]` | `[9 5]` | unary_sign_lists |
| tests/lists/test_list_literal_sign.pairs.warp:7 | `a=5; [a -1]` | `[5, -1]` | `[4]` | unary_sign_lists |
| tests/lists/test_lists.pairs.warp:8 | `pixel=[1 2 4];pixel number` | `3` | `[1 2 4] class number` | other |
| tests/lists/test_lists.pairs.warp:61 | `(1;2;3)` | `[1, 2, 3]` | `3` | list_literal_as_block |
| tests/lists/test_lists.pairs.warp:62 | `1,2,3` | `[1, 2, 3]` | `3` | list_literal_as_block |
| tests/lists/test_lists.pairs.warp:66 | `{1 2 3}` | `[1, 2, 3]` | `3` | list_literal_as_block |
| tests/lists/test_lists.pairs.warp:67 | `{1,2,3}` | `[1, 2, 3]` | `3` | list_literal_as_block |
| tests/lists/test_lists.pairs.warp:68 | `{1;2;3}` | `[1, 2, 3]` | `3` | list_literal_as_block |
| tests/lists/test_map_starts.pairs.warp:9 | `p = {x:1}; p.@source = \"gps\"; #p` | `1` | `2` | meta_attributes |
| tests/lists/test_map_starts.warp:4 | `fill(m, n) := { for i in 0..n { m[\"k\\(i)\"] = i }; count(m) }; fill({}, 300000)` | `300000` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_map_variables.warp:4 | `m = {}; for i in 0..2000000 { m[\"k\\(i)\"] = i }; s = 0; for i in 0..2000000 { s += m[\"k\\(i)\"] }; s + count(m)` | `(1999999000000+2000000)` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_map_variables.warp:7 | `m = {}; for i in 1 to 100000 { m[i] = i * 2 }; count(keys(m)) + count(values(m))` | `200000` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_map_variables.warp:8 | `m = {}; for i in 1 to 100000 { m[i] = i * 2 }; values(m)#100000 + count(keys(m))` | `300000` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_min_max.pairs.warp:20 | `min(a,b):=a*b; min(2,3)` | `2` | `6` | builtin_shadowing |
| tests/lists/test_node_lists.warp:9 | `def qs(arr, lo, hi) { if lo >= hi { return arr }; pivot = arr[hi]; i = lo; for j in lo..hi { if arr[j] < pivot { t = arr` | `5` | `1` | for_loop_iteration |
| tests/lists/test_range_arguments.pairs.warp:5 | `f(xs) := count xs; f(1 to 1000000000000)` | `1000000000000` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_range_arguments.pairs.warp:6 | `f(xs) := xs#3; f(5..1000000000)` | `7` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_range_arguments.pairs.warp:12 | `f(xs) := xs.count; xs = 1..1000000000000; f(xs)` | `999999999999` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_range_arguments.warp:4 | `h(zs) := zs.count; g(ys) := h(ys); f(xs) := g(xs); f(1..1000000000000)` | `999999999999` | `timed out after 30 s` | timeout_or_crash |
| tests/lists/test_range_arguments.warp:5 | `g(a, b) := a.count; f(xs) := g([1], xs) + g(xs, [1]); f(1..1000000000000)` | `1000000000000` | `crashed (exit status: 1): memory cap: growing a memory to 4096 MB passes the cap of 2048 MB (WARP_MEMORY_CAP_MB)` | timeout_or_crash |
| tests/lists/test_range_values.pairs.warp:7 | `type(1..5)` | `"list of int"` | `class list` | type_values |
| tests/lists/test_signed_operand_list.pairs.warp:4 | `1 -1` | `[1, -1]` | `0` | unary_sign_lists |
| tests/lists/test_signed_operand_list.pairs.warp:5 | `[1 -1]` | `[1, -1]` | `[0]` | unary_sign_lists |
| tests/lists/test_signed_operand_list.pairs.warp:6 | `[1 -1 2]` | `[1, -1, 2]` | `[0 2]` | unary_sign_lists |
| tests/lists/test_signed_operand_list.pairs.warp:7 | `[1 +1]` | `[1, 1]` | `[2]` | unary_sign_lists |
| tests/lists/test_signed_operand_list.pairs.warp:18 | `[3 -1]` | `[3, -1]` | `[2]` | unary_sign_lists |
| tests/lists/test_type_word_items.pairs.warp:7 | `int 4.7` | `4` | `class int 4.7` | prefix_word_call |
| tests/lists/test_unpacking.pairs.warp:53 | `a, b = \"hi\"; a` | `'h'` | `"h"` | char_vs_text |
| tests/lists/test_unpacking_into_existing.pairs.warp:7 | `x = 0.5; x, y = [2, 3]; x + y` | `5` | `5` | num_int_vs_float |
| tests/modules/test_english_phrases.pairs.warp:4 | `data(set width of p to 12) == parse_code(\"set width of p to 12\")` | `yes` | `no` | code_data_nodes |
| tests/modules/test_json_parse_form.warp:4 | `use json\nm = parse(\"{\\\"a\\\": 1}\")\nm is map` | `yes` | `no` | code_data_nodes |
| tests/modules/test_module_function_dir.warp:8 | `use math\nstr(type(is_square))` | `"function: (n: int) -> bool"` | `"class function"` | type_of_function_signature |
| tests/modules/test_packages.pairs.warp:7 | `module_directory` | `'.'` | `"/Volumes/Worktrees/measure-pure/tests/modules"` | other |
| tests/modules/test_prelude_shadowing.pairs.warp:4 | `round_to(x, d) := 99; round(2.555, 2)` | `2.56` | `99` | builtin_shadowing |
| tests/modules/test_prelude_shadowing.pairs.warp:6 | `first_items(x, n) := 99; [5, 6, 7].first(2)` | `[5, 6]` | `99` | builtin_shadowing |
| tests/modules/test_std_import_forms.pairs.warp:8 | `import sqrt from math; sqrt 4` | `2` | `2` | num_int_vs_float |
| tests/modules/test_std_list.pairs.warp:53 | `use list; stdev([2, 4, 4, 4, 5, 5, 7, 9])` | `2` | `2` | num_int_vs_float |
| tests/modules/test_std_map.pairs.warp:4 | `use map; m = invert({a:\"x\" b:\"y\"}); m.get(\"y\")` | `'b'` | `"b"` | char_vs_text |
| tests/modules/test_std_prelude.pairs.warp:21 | `first(x) := 7; first([1])` | `1` | `7` | builtin_shadowing |
| tests/modules/test_use_modules.pairs.warp:16 | `use draw; global hue = 10; c = hsv(120, 1, 1); hue` | `10` | `2` | other |
| tests/modules/test_wasm_modules.pairs.warp:11 | `include tests/fixtures/wasm/fourty_two` | `42` | `ø` | other |
| tests/node/test_grouped_parts.pairs.warp:7 | `x = data [(a * 2)]; (x#1).bracket` | `'('` | `"("` | char_vs_text |
| tests/node/test_list_separator.pairs.warp:4 | `c = data [a, b]; c.separator` | `','` | `","` | char_vs_text |
| tests/node/test_list_separator.pairs.warp:5 | `c = data (a; b); c.separator` | `';'` | `";"` | char_vs_text |
| tests/node/test_list_separator.pairs.warp:6 | `c = data [a b]; c.separator` | `' '` | `" "` | char_vs_text |
| tests/node/test_list_separator.pairs.warp:7 | `c = data [a]; c.separator` | `""` | `" "` | code_data_nodes |
| tests/node/test_node_api.pairs.warp:5 | `p = data (x: 3); p.op` | `':'` | `":"` | char_vs_text |
| tests/node/test_node_api.pairs.warp:6 | `k = data a+b; k.op` | `'+'` | `"+"` | char_vs_text |
| tests/node/test_node_api.pairs.warp:7 | `p = data (x: 3); p.name` | `'x'` | `"x"` | char_vs_text |
| tests/node/test_node_api.pairs.warp:13 | `c = data f(1, 2); c.bracket` | `'('` | `"("` | char_vs_text |
| tests/node/test_node_api.pairs.warp:14 | `c = data [a b]; c.bracket` | `'['` | `"["` | char_vs_text |
| tests/node/test_node_api.pairs.warp:15 | `c = data {a b}; c.bracket` | `'{'` | `"{"` | char_vs_text |
| tests/node/test_node_api.pairs.warp:18 | `g = bracketed([1, 2], \"(\"); g.bracket` | `'('` | `"("` | char_vs_text |
| tests/node/test_node_api.pairs.warp:25 | `bracketed([1, 2], \"(\") == [1, 2]` | `no` | `yes` | other |
| tests/node/test_node_cases.pairs.warp:4 | `n = parse(\"x\"); match n { Symbol(name): name, _: 0 }` | `'x'` | `"x"` | char_vs_text |
| tests/node/test_node_cases.pairs.warp:7 | `List([1, 2], Bracket.Round, Separator.Comma)` | `(1, 2)` | `(1, 2)` | same_print_other |
| tests/node/test_node_cases.pairs.warp:15 | `Op.Add as text` | `'+'` | `"+"` | char_vs_text |
| tests/node/test_node_cases.pairs.warp:16 | `Bracket.Square as text` | `'['` | `"["` | char_vs_text |
| tests/node/test_node_cases.pairs.warp:17 | `Separator.Comma as text` | `','` | `","` | char_vs_text |
| tests/node/test_node_cases.pairs.warp:25 | `n = parse(\"a+1\"); match n { Key(l, op, r): op as text, _: 0 }` | `'+'` | `"+"` | char_vs_text |
| tests/node/test_node_cases.warp:14 | `swap(n: Node): Node := match n { Key(l, Op.Add, r): Key(r, Op.Add, l), _: n }; swap(parse(\"1+2\"))` | `2+1` | `2+1` | num_int_vs_float |
| tests/node/test_node_cases.warp:23 | `n = parse(\"(a)\"); match n { List(items, bracket, s): bracket as text, _: 0 }` | `'('` | `"("` | char_vs_text |
| tests/node/test_parse_word.pairs.warp:5 | `x = parse \"a + 0\"; x.op` | `'+'` | `"+"` | char_vs_text |
| tests/node/test_parse_word.pairs.warp:10 | `(data (n * 1)).op` | `'*'` | `"*"` | char_vs_text |
| tests/node/test_parse_word.pairs.warp:31 | `t = \"a.warp\"; parse(t).op` | `'.'` | `"."` | char_vs_text |
| tests/node/test_parse_word.pairs.warp:40 | `x = data (a; b); (x#2).name` | `'b'` | `"b"` | char_vs_text |
| tests/node/test_parsed_node_walks.pairs.warp:13 | `parse(\"f(x)\").bracket` | `'('` | `"("` | char_vs_text |
| tests/node/test_parsed_node_walks.pairs.warp:17 | `k = key(symbol(\"y\"), \"+\", parse(\"z\")); \"\\(k.value)\"` | `'z'` | `"z"` | char_vs_text |
| tests/node/test_parsed_node_walks.pairs.warp:37 | `k = key(symbol(\"op\"), \":\", 3); op_of(k)` | `':'` | `":"` | char_vs_text |
| tests/node/test_parsed_node_walks.pairs.warp:41 | `separator_of(parse(\"[a, b]\"))` | `','` | `","` | char_vs_text |
| tests/node/test_quote_as_written.pairs.warp:4 | `q = data (a.b); q.op` | `'.'` | `"."` | char_vs_text |
| tests/numbers/test_angle.pairs.warp:20 | `4.0 and 5.0` | `5` | `5` | num_int_vs_float |
| tests/numbers/test_angle.pairs.warp:26 | `4 and 5.0` | `5` | `5` | num_int_vs_float |
| tests/numbers/test_big_int_bits.pairs.warp:18 | `2^62 \| 2^61` | `6917529027641081856` | `4611686018427387904` | other |
| tests/numbers/test_big_int_text.pairs.warp:9 | `text(0)` | `'0'` | `"0"` | char_vs_text |
| tests/numbers/test_counting_units.pairs.warp:11 | `size of \"äb\" in bytes` | `3` | `no` | for_loop_iteration |
| tests/numbers/test_exact_parameters.pairs.warp:10 | `float64 ratio = 1.5; ratio * 2` | `3` | `3` | num_int_vs_float |
| tests/numbers/test_fast_scope.warp:14 | `fast\nx = 0.5\nx * 2` | `1` | `1` | num_int_vs_float |
| tests/numbers/test_float_exact_context.pairs.warp:12 | `xs = []; xs.add(√2 * √2 == 2); xs#1` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:13 | `xs = []; r = √2 * √2; r == 2` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:14 | `xs = []; r = √2 * √2; ok = r == 2; xs.add(ok); xs#1` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:31 | `exact x = √2; x * x` | `2` | `2.0000000000000004` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:32 | `real x = sqrt(2); x * x == 2` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:33 | `exact x = √2; x as text` | `"√2"` | `"1.4142135623730951"` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:34 | `real == exact` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:37 | `√2 is exact` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:38 | `type(√2) == real` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_float_exact_context.pairs.warp:44 | `type(sqrt(2.0))` | `"real"` | `class float` | exact_real_symbolic |
| tests/numbers/test_float_quotient.pairs.warp:6 | `n = 3 + random_below(1); float y = (2^53 + 1) / n; y` | `3.002399751580331e15` | `3002399751580331` | other |
| tests/numbers/test_float_to_int_range.pairs.warp:6 | `round(2.5)` | `2` | `2` | num_int_vs_float |
| tests/numbers/test_float_to_int_range.pairs.warp:7 | `round(3.5)` | `4` | `4` | num_int_vs_float |
| tests/numbers/test_implicit_libm.pairs.warp:15 | `def sin(x) := 42; sin(1.0)` | `0.8414709848078965` | `42` | builtin_shadowing |
| tests/numbers/test_long_decimals.pairs.warp:4 | `text(4.4215251521521521512)` | `"4.4215251521521521512"` | `"4.421525152152152"` | other |
| tests/numbers/test_long_decimals.pairs.warp:6 | `type(1.0000000000000000001)` | `"rational"` | `class int` | type_values |
| tests/numbers/test_math.pairs.warp:45 | `√4` | `2` | `2` | num_int_vs_float |
| tests/numbers/test_math.pairs.warp:102 | `sqrt 9` | `3` | `3` | num_int_vs_float |
| tests/numbers/test_math.pairs.warp:103 | `sqrt 9.0` | `3` | `3` | num_int_vs_float |
| tests/numbers/test_method_on_operator_call.pairs.warp:4 | `sqrt(2).round(2)` | `1.41` | `1.4142135623730951` | exact_real_symbolic |
| tests/numbers/test_nearest.pairs.warp:4 | `nearest(sqrt(2), rational) == 941664/665857` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_nearest.pairs.warp:28 | `sqrt(2).nearest(rational, within: 0.001) == 41/29` | `yes` | `no` | exact_real_symbolic |
| tests/numbers/test_nearest.pairs.warp:29 | `type(sqrt(2).nearest(rational))` | `"rational"` | `class float` | exact_real_symbolic |
| tests/numbers/test_repeating_decimal.pairs.warp:10 | `0.9\u{305}` | `1` | `0 "9̅"` | other |
| tests/numbers/test_round_to_digits.pairs.warp:9 | `round(7.5)` | `8` | `8` | num_int_vs_float |
| tests/numbers/test_rounding_in_functions.pairs.warp:4 | `f(x:float) := floor(x); f(2.5)` | `2` | `2` | num_int_vs_float |
| tests/numbers/test_rounding_in_functions.pairs.warp:6 | `f(x:float) := round(x); f(2.5)` | `2` | `2` | num_int_vs_float |
| tests/numbers/test_rounding_in_functions.pairs.warp:7 | `f(x:float) := floor(x) + 1; f(2.5)` | `3` | `3` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:4 | `n = 7; round(2^(n/12) * 1000)` | `1498` | `1498` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:5 | `semitones(n) := 2^(n/12); round(440 * semitones(7))` | `659` | `659` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:11 | `pick(xs) := { f = xs#1; f ^ 2 }; round(pick([sqrt(2)]) * 1000)` | `2000` | `2000` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:16 | `pick(xs) := { f = xs#1; f * 1 }; round(pick([sqrt(2)]) * 1000)` | `1414` | `1414` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:17 | `pick(xs) := { f = xs#1; f * 1 }; round(pick([7]))` | `7` | `7` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:20 | `ratio(k) := 2^((k - 69) / 12); round(abs(ratio(57)) * 1000)` | `500` | `500` | num_int_vs_float |
| tests/numbers/test_run_time_power.pairs.warp:25 | `l = -0.45; round(10^l * 1000)` | `355` | `355` | num_int_vs_float |
| tests/numbers/test_run_time_remainder.pairs.warp:5 | `head_of(xs) := { f = xs#1; 7 % f }; head_of([2.5])` | `2` | `2` | num_int_vs_float |
| tests/numbers/test_run_time_remainder.warp:7 | `ratio = 1.0594630943592953; phases(xs) := [f % 1 for f in xs]; round(phases([440.0 * ratio])#1 * 1000)` | `164` | `164` | num_int_vs_float |
| tests/numbers/test_superscript_variables.pairs.warp:5 | `n=3.0;2.0ⁿ` | `8` | `8` | num_int_vs_float |
| tests/numbers/test_unbounded_int.pairs.warp:27 | `x=-9223372036854775808; -x` | `9223372036854775808` | `-9223372036854775808` | other |
| tests/numbers/test_unbounded_int.pairs.warp:28 | `x=-9223372036854775808; abs(x)` | `9223372036854775808` | `-9223372036854775808` | other |
| tests/numbers/test_unbounded_int.pairs.warp:67 | `s = 0; for k in 1..1000 { s += 1/k }; floor(s * 1000)` | `7484` | `-5` | for_loop_iteration |
| tests/numbers/test_unit_beside_variable.pairs.warp:4 | `use units; m = 5; str(3m)` | `"3m"` | `"15"` | units |
| tests/numbers/test_unit_beside_variable.warp:4 | `use units\ns = 2\nstr(0.5s)` | `"0.5s"` | `"1"` | units |
| tests/numbers/test_unit_definitions.warp:25 | `class N{name: text}\na = N(\"a\")\na.name` | `'a'` | `"a"` | char_vs_text |
| tests/numbers/test_units_after_numbers.warp:4 | `use units\ns = 3\nstr(0.5s)` | `"0.5s"` | `"1.5"` | units |
| tests/numbers/test_units_after_numbers.warp:7 | `use units\ns = 3\nstr(2 * 0.5s)` | `"1s"` | `"3"` | units |
| tests/numbers/test_units_arithmetic.pairs.warp:4 | `use units;m=5;str(3m)` | `"3m"` | `"15"` | units |
| tests/operators/test_bit_operators_on_ints.pairs.warp:21 | `n = 2; m = 4; n \| m` | `6` | `2` | other |
| tests/operators/test_braceless_operator_words.pairs.warp:4 | `sqrt 4 + 5` | `3` | `3` | num_int_vs_float |
| tests/operators/test_braceless_operator_words.pairs.warp:7 | `sqrt 2 * 8` | `4` | `4` | num_int_vs_float |
| tests/operators/test_braceless_operator_words.pairs.warp:11 | `1 + sqrt 4 + 5` | `4` | `4` | num_int_vs_float |
| tests/operators/test_braceless_operator_words.pairs.warp:14 | `sqrt(16) + 1` | `5` | `5` | num_int_vs_float |
| tests/operators/test_braceless_operator_words.pairs.warp:18 | `√4 + 5` | `7` | `7` | num_int_vs_float |
| tests/operators/test_logical_values_in_functions.pairs.warp:6 | `s(a) := a or \"d\"; s(\"x\")` | `'x'` | `"x"` | char_vs_text |
| tests/operators/test_mutating_bang.pairs.warp:12 | `xs=[1 2 3]; reverse xs!; xs#1` | `3` | `1` | other |
| tests/operators/test_mutating_bang.pairs.warp:14 | `twice:=it*2; x=3; twice x!; x` | `6` | `3` | it_lambda_recursion |
| tests/operators/test_operator_spellings.pairs.warp:33 | `x = yes; x and= no; x` | `no` | `yes` | other |
| tests/operators/test_operator_spellings.pairs.warp:34 | `x = no; x or= yes; x` | `yes` | `no` | other |
| tests/operators/test_operators.pairs.warp:6 | `while True : 2` | `0` | `timed out after 30 s` | timeout_or_crash |
| tests/operators/test_pipe_stages.pairs.warp:11 | `-4\|abs` | `4` | `-4` | other |
| tests/operators/test_pipeline.warp:12 | `square := it*it; numbers=[1 2 3 4]; square numbers \|> filter(x => x > 5) \|> sum` | `25` | `0` | it_lambda_recursion |
| tests/operators/test_print_bang.pairs.warp:6 | `xs = [3 1 2]; sort xs!; xs#1` | `1` | `3` | other |
| tests/operators/test_suffix_power_places.pairs.warp:10 | `x = -3; √x²` | `3` | `3` | num_int_vs_float |
| tests/operators/test_suffix_power_places.pairs.warp:11 | `x = 9; √x^2` | `9` | `9` | num_int_vs_float |
| tests/operators/test_suffix_power_places.pairs.warp:12 | `x = 4; √x*2` | `4` | `4` | num_int_vs_float |
| tests/operators/test_suffix_power_places.pairs.warp:13 | `p = {x: 4}; √p.x` | `2` | `2` | num_int_vs_float |
| tests/operators/test_then_pipe.pairs.warp:11 | `x = 3; x < 2 then 1 else 2` | `2` | `1` | other |
| tests/parser/test_and_after_comparison.pairs.warp:5 | `sq(x:int) := x*x; sq 2 == 4 and sq 3 == 9` | `yes` | `0` | other |
| tests/parser/test_juxtaposition.pairs.warp:5 | `x=4;0.5x` | `2` | `2` | num_int_vs_float |
| tests/parser/test_meta_attributes.pairs.warp:13 | `p = {x:1 @unit:\"cm\"}; p@unit` | `"cm"` | `{x:1}` | meta_attributes |
| tests/parser/test_meta_attributes.pairs.warp:23 | `p = {x:1}; p.@s = 30; t=0; for k,v in p {t+=v}; t` | `1` | `31` | meta_attributes |
| tests/parser/test_meta_attributes.pairs.warp:27 | `p = {x:1}; p.@s = 30; p.z = 5; #p.keys` | `2` | `3` | meta_attributes |
| tests/parser/test_newline_precedence.warp:18 | `fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)` | `56` | `55` | it_lambda_recursion |
| tests/parser/test_operator_aliases.warp:6 | `prefix operator ∂ := sqrt\n∂16` | `4` | `"∂" 16` | other |
| tests/parser/test_parser_sugar.warp:16 | `1 + \\\n2` | `3` | `2` | other |
| tests/parser/test_parser_sugar.warp:20 | `x = 1 + \\\n  2 + \\\n  3; x` | `6` | `"1\\"` | other |
| tests/parser/test_spaced_optional_type.pairs.warp:6 | `name: text? = \"a\"; name` | `'a'` | `"a"` | char_vs_text |
| tests/parser/test_statement_phrases.pairs.warp:4 | `f := it*2; f 3 and f 5` | `10` | `20` | it_lambda_recursion |
| tests/parser/test_surface_syntax.pairs.warp:17 | `¬ false` | `yes` | `no no` | logic_rpn_negation |
| tests/parser/test_surface_syntax.pairs.warp:18 | `√16` | `4` | `4` | num_int_vs_float |
| tests/parser/test_unary_plus.pairs.warp:4 | `+5` | `5` | `"+" 5` | unary_sign_lists |
| tests/parser/test_unary_plus.pairs.warp:5 | `+2.5` | `2.5` | `"+" 2.5` | unary_sign_lists |
| tests/parser/test_unary_plus.pairs.warp:7 | `3 + +2` | `5` | `"3+" 2` | unary_sign_lists |
| tests/parser/test_unary_plus.pairs.warp:8 | `x=4; +x` | `4` | `"+" 4` | unary_sign_lists |
| tests/parser/test_unary_plus.pairs.warp:9 | `+(2*3)` | `6` | `"+" 6` | unary_sign_lists |
| tests/parser/test_unary_plus.pairs.warp:10 | `[+1 -2]` | `[1, -2]` | `['+' -1]` | unary_sign_lists |
| tests/programs/test_calculator_fixes.warp:24 | `def f(n, x) { if n == \"s\" { return x * 2.5 }\nreturn error(\"unknown: \" + n) }\nf(\"s\", 2)` | `5` | `5` | num_int_vs_float |
| tests/programs/test_calculator_fixes.warp:29 | `def f(x) { y = x * 1.5\nwhile true { if y > 10 { return y }\ny = y * 2 } }\nf(2)` | `12` | `12` | num_int_vs_float |
| tests/programs/test_json_parser_fixes.pairs.warp:9 | `m = {a: 5}; str(m.a)` | `'5'` | `"5"` | char_vs_text |
| tests/scope/test_global_node_widening.pairs.warp:4 | `f(l) := 10^(l/20.0); global g = 0.3; g = f(-9); round(g * 1000)` | `355` | `355` | num_int_vs_float |
| tests/scope/test_global_node_widening.warp:4 | `f(l) := 10^(l/20.0); global g = 0.3; h() := g*2; g = f(-9); round(h() * 1000)` | `710` | `710` | num_int_vs_float |
| tests/scope/test_global_node_widening.warp:5 | `f(l) := 10^(l/20.0); global g = 0.3; set() := { g = f(-9) }; set(); round(g * 1000)` | `355` | `355` | num_int_vs_float |
| tests/scope/test_parameter_shadows_function.pairs.warp:9 | `apply(f, x) := f x; inc(a) := a + 1; apply(inc, 1)` | `2` | `inc 1` | fn_value_not_applied |
| tests/scope/test_prefixed_declarations.pairs.warp:6 | `float f = 2; f` | `2` | `2` | num_int_vs_float |
| tests/scope/test_variable_kind_clash.pairs.warp:5 | `x = \"a\"; x = 'b'; x` | `'b'` | `"b"` | char_vs_text |
| tests/sweeps/test_todo.pairs.warp:7 | `3 + √9` | `6` | `6` | num_int_vs_float |
| tests/text/test_add_to_text.pairs.warp:4 | `x=\"ab\"; add \"c\" to x; x` | `"abc"` | `"ab['c']"` | other |
| tests/text/test_add_to_text.pairs.warp:5 | `x=\"ab\"; add \"cd\" to x; x` | `"abcd"` | `"ab[\"cd\"]"` | other |
| tests/text/test_add_to_text.pairs.warp:6 | `x=\"ab\"; x.add(\"c\"); x` | `"abc"` | `"ab['c']"` | other |
| tests/text/test_add_to_text.pairs.warp:7 | `x=\"a\"; x = x + \"b\"; add \"c\" to x; x` | `"abc"` | `"ab['c']"` | other |
| tests/text/test_char_parameter_type.pairs.warp:4 | `def g(t) := type(t); g(\"a\")` | `"codepoint"` | `class text` | type_values |
| tests/text/test_char_parameter_type.pairs.warp:5 | `f(t:text) := type(t); f(\"a\")` | `"codepoint"` | `class text` | type_values |
| tests/text/test_char_parameter_type.pairs.warp:6 | `def g(t) := type(t); g(\"ä\")` | `"codepoint"` | `class text` | type_values |
| tests/text/test_character_comparison.pairs.warp:35 | `try ord(\"ab\") else 0` | `0` | `97` | other |
| tests/text/test_constant_expression_text.pairs.warp:4 | `str(1+2)` | `'3'` | `"3"` | char_vs_text |
| tests/text/test_constant_expression_text.pairs.warp:5 | `(1+2) as string` | `'3'` | `"3"` | char_vs_text |
| tests/text/test_constant_expression_text.pairs.warp:6 | `\"\" + (1+2)` | `'3'` | `"3"` | char_vs_text |
| tests/text/test_declared_text_one_character.pairs.warp:4 | `string x = \"a\"; x` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_declared_text_one_character.pairs.warp:6 | `text x = \"a\"; x` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_interpolation.pairs.warp:6 | `\"\\(1+2)\"` | `'3'` | `"3"` | char_vs_text |
| tests/text/test_library_unicode.pairs.warp:16 | `x=[2.5, -1.0, 0.5]; (x.sort)#1` | `-1` | `-1` | num_int_vs_float |
| tests/text/test_one_character_argument.pairs.warp:6 | `g(t) := t; x = \"3\"; g(x)` | `'3'` | `"3"` | char_vs_text |
| tests/text/test_str_computed.pairs.warp:4 | `str({a:\"x\"}.a)` | `'x'` | `"x"` | char_vs_text |
| tests/text/test_str_computed.pairs.warp:5 | `str([\"a\", \"b\"]#2)` | `'b'` | `"b"` | char_vs_text |
| tests/text/test_str_computed.warp:6 | `class P{x:number}\nstr(P(5).x)` | `'5'` | `"5"` | char_vs_text |
| tests/text/test_string.pairs.warp:4 | `3.0` | `3` | `3` | num_int_vs_float |
| tests/text/test_string.pairs.warp:23 | ``${1+1}`` | `'2'` | `"`" "$" 2 "`"` | unary_sign_lists |
| tests/text/test_string.pairs.warp:59 | `char 1 in 'world'` | `'w'` | `class char no` | prefix_word_call |
| tests/text/test_text_bytes.pairs.warp:8 | `s=\"\"; s = s + \"𓀀\"; s` | `'𓀀'` | `"𓀀"` | char_vs_text |
| tests/text/test_text_concat.pairs.warp:8 | `\"\"+\"x\"` | `'x'` | `"x"` | char_vs_text |
| tests/text/test_text_plus_printable.warp:4 | `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\n\"hi \" + bo` | `"hi person Bo"` | `"hi P{name:\"Bo\"}"` | custom_text_printer |
| tests/text/test_text_plus_printable.warp:8 | `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\n\"hi \" + bo + \"!\"` | `"hi person Bo!"` | `"hi P{name:\"Bo\"}!"` | custom_text_printer |
| tests/text/test_text_plus_printable.warp:12 | `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\nbo + \" there\"` | `"person Bo there"` | `"P{name:\"Bo\"} there"` | custom_text_printer |
| tests/text/test_text_plus_printable.warp:16 | `class P{name: text}\ntext(p:P) := \"person \" + p.name\nbo = P(\"Bo\")\ns = \"yo \"\ns + bo` | `"yo person Bo"` | `"yo P{name:\"Bo\"}"` | custom_text_printer |
| tests/text/test_text_runtime.pairs.warp:24 | `x=\"a\"; reverse x` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_text_type_words.pairs.warp:9 | `str(√2)` | `"√2"` | `"1.4142135623730951"` | exact_real_symbolic |
| tests/text/test_text_type_words.warp:4 | `class person{name:text}; text(p:person) := \"P \" + p.name; x = person{name:\"a\"}; x as string` | `"P a"` | `"person{name:'a'}"` | custom_text_printer |
| tests/text/test_text_unit_lists.pairs.warp:7 | `for byte in \"äb\" { byte }` | `98` | `"b"` | for_loop_iteration |
| tests/text/test_text_variable_assignment.pairs.warp:4 | `x=\"a\"; x` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_text_variable_assignment.pairs.warp:5 | `y=\"a\"; x=y; x` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_text_variable_assignment.pairs.warp:8 | `x=\"a\"; x=\"b\"; x` | `'b'` | `"b"` | char_vs_text |
| tests/text/test_text_variable_assignment.pairs.warp:9 | `x=\"a\"; if 1 {x=\"b\"}; x` | `'b'` | `"b"` | char_vs_text |
| tests/text/test_text_variable_assignment.pairs.warp:10 | `x='a'; if 1 {x='b'}; x` | `'b'` | `"b"` | char_vs_text |
| tests/text/test_trim.pairs.warp:11 | `strip(\" a \")` | `'a'` | `"a"` | char_vs_text |
| tests/text/test_unicode_escape.pairs.warp:4 | `'\\u{e9}'=='e\\u{301}'` | `1` | `no` | other |
| tests/types/test_any_into_annotated_types.pairs.warp:6 | `y: any = \"a\"; f(s: text) := s; f(y)` | `'a'` | `"a"` | char_vs_text |
| tests/types/test_cast_bugs.pairs.warp:8 | `x=3; x as string` | `'3'` | `"3"` | char_vs_text |
| tests/types/test_cast_bugs.pairs.warp:10 | `x=3 as string; x` | `'3'` | `"3"` | char_vs_text |
| tests/types/test_checked_call_kinds.pairs.warp:4 | `f(x: text) := x; f(\"a\")` | `'a'` | `"a"` | char_vs_text |
| tests/types/test_class_json.pairs.warp:4 | `use json; class Point{x:int y:int}; to_json(Point(1, 2))` | `"{\"x\":1,\"y\":2}"` | `"{\"Point\":{\"x\":1,\"y\":2}}"` | class_json |
| tests/types/test_class_json.warp:4 | `use json; class Point{x:int y:int}; to_json([Point(1, 2), Point(3, 4)])` | `"[{\"x\":1,\"y\":2},{\"x\":3,\"y\":4}]"` | `"[{\"Point\":{\"x\":1,\"y\":2}},{\"Point\":{\"x\":3,\"y\":4}}]"` | class_json |
| tests/types/test_class_json.warp:5 | `use json; class Point{x:int y:int}; class Line{a:Point b:Point}; to_json(Line(Point(1, 2), Point(3, 4)))` | `"{\"a\":{\"x\":1,\"y\":2},\"b\":{\"x\":3,\"y\":4}}"` | `"{\"Line\":{\"a\":{\"Point\":{\"x\":1,\"y\":2}},\"b\":{\"Point\":{\"x\":3,\"y\":4}}}}"` | class_json |
| tests/types/test_class_properties.pairs.warp:4 | `class P {x: int}; f(p: P, v) := P(v * 2); p = P(1); p.f = 5; p.x` | `10` | `1` | other |
| tests/types/test_code_type.pairs.warp:22 | `type parse(\"1+1\")` | `"code"` | `class type 1+1` | prefix_word_call |
| tests/types/test_code_type.pairs.warp:25 | `parse(\"1+1\").op` | `'+'` | `"+"` | char_vs_text |
| tests/types/test_copies.warp:4 | `class P { x: int }; p = P(1); ps = [p]; qs = ps.copy(); qs#1.x = 9; p.x` | `1` | `9` | other |
| tests/types/test_declared_list_marks.warp:4 | `names: texts = [\"hi\"]; ys = names; try { ys.add(1) } catch e { \"caught\" }` | `"caught"` | `["hi" 1]` | other |
| tests/types/test_dispatch.pairs.warp:7 | `def sort(x):=0; sort([2,1]) == [1 2]` | `yes` | `no` | builtin_shadowing |
| tests/types/test_dispatch.pairs.warp:15 | `class Bag{items}; def sum(b:Bag):=7; sum(Bag([1])) + sum([1,2])` | `10` | `14` | builtin_shadowing |
| tests/types/test_empty_type_aliases.pairs.warp:20 | `type(ø) == empty` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:21 | `type(ø) == Empty` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:22 | `type(ø) == unit` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:23 | `x = ø; type(x) == nil` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:25 | `type(\"ab\") == string` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:29 | `exact == real` | `yes` | `no` | exact_real_symbolic |
| tests/types/test_empty_type_aliases.pairs.warp:30 | `integer == int` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:31 | `string == type(\"ab\")` | `yes` | `no` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:32 | `int != integer` | `no` | `yes` | other |
| tests/types/test_empty_type_aliases.pairs.warp:33 | `type(\"ab\") != string` | `no` | `yes` | type_values |
| tests/types/test_empty_type_aliases.pairs.warp:35 | `unit == type(ø)` | `yes` | `no` | type_values |
| tests/types/test_float_fields.pairs.warp:13 | `def f() { return 1.5 }; f() * 2.0` | `3` | `3` | num_int_vs_float |
| tests/types/test_float_fields.pairs.warp:17 | `def g() := 2.5; x = g(); def f() { return x * 2 }; f()` | `5` | `5` | num_int_vs_float |
| tests/types/test_float_fields.warp:4 | `type V { x: float, y: float }; def dot(a, b) := a.x * b.x + a.y * b.y; dot(V(1.5, 2), V(2, 1))` | `5` | `5` | num_int_vs_float |
| tests/types/test_is_declaration.pairs.warp:7 | `x=9; x is number 9` | `1` | `no` | other |
| tests/types/test_is_declaration.pairs.warp:9 | `f(x) := x is number 3; f(3)` | `1` | `no` | other |
| tests/types/test_iterable.warp:8 | `class bag{a:int b:int}; iterate(g:bag) := [g.a, g.b]; g = bag(5, 6); 6 in g` | `yes` | `no` | other |
| tests/types/test_nested_type_calls.pairs.warp:6 | `int float 3` | `3` | `class int class float 3` | prefix_word_call |
| tests/types/test_nested_type_calls.pairs.warp:7 | `text int 3.7` | `'3'` | `class text class int 3.7` | prefix_word_call |
| tests/types/test_nested_type_calls.pairs.warp:10 | `f(x) := x*2; text f 4` | `'8'` | `class text f 4` | prefix_word_call |
| tests/types/test_nested_type_calls.pairs.warp:14 | `int 3 == 3` | `yes` | `class int yes` | prefix_word_call |
| tests/types/test_nested_type_calls.pairs.warp:15 | `int 3.7 == 3` | `yes` | `class int no` | prefix_word_call |
| tests/types/test_nested_type_calls.pairs.warp:17 | `int 3 == 4` | `no` | `class int no` | prefix_word_call |
| tests/types/test_number_converts.pairs.warp:4 | `class P { v: float }; P(1/3).v is float` | `yes` | `no` | other |
| tests/types/test_number_converts.pairs.warp:7 | `xs: floats = [1/3, 2]; xs[0] is float` | `yes` | `no` | other |
| tests/types/test_number_converts.pairs.warp:9 | `class Q { xs: floats }; Q([1/3]).xs[0] is float` | `yes` | `no` | other |
| tests/types/test_number_field.warp:16 | `class Q{amount:number}\nQ(2.5).amount * 2` | `5` | `5` | num_int_vs_float |
| tests/types/test_one_dispatch.pairs.warp:5 | `p = {first: 1}; p.first()` | `1` | `first:1` | other |
| tests/types/test_one_dispatch.pairs.warp:7 | `f(m) := m.first(); f({first: 9})` | `9` | `first:9` | other |
| tests/types/test_one_dispatch.pairs.warp:53 | `upper(t) := \"U\"; upper(\"a\")` | `'A'` | `"U"` | builtin_shadowing |
| tests/types/test_one_dispatch.pairs.warp:60 | `add(t, n) := 5; xs=[1]; add(xs, 2); xs == [1 2]` | `yes` | `no` | builtin_shadowing |
| tests/types/test_one_dispatch.pairs.warp:63 | `sum(xs) := 7; t=[1 2]; t.sum` | `3` | `7` | builtin_shadowing |
| tests/types/test_printable.warp:5 | `class person{name:text}; text(p:person) := \"P \" + p.name; x = person{name:\"a\"}; str(x)` | `"P a"` | `"person{name:'a'}"` | custom_text_printer |
| tests/types/test_printable.warp:10 | `class person{name:text}; text(p:person) := \"P \" + p.name; x = person{name:\"a\"}; \"is \\(x)\"` | `"is P a"` | `"is person{name:'a'}"` | custom_text_printer |
| tests/types/test_real_text.pairs.warp:4 | `\"f\" + sqrt(2)` | `"f√2"` | `"f1.4142135623730951"` | exact_real_symbolic |
| tests/types/test_real_text.pairs.warp:5 | `√2 + \"x\"` | `"√2x"` | `"1.4142135623730951x"` | exact_real_symbolic |
| tests/types/test_real_text.pairs.warp:7 | `x=√2*3; \"v=\" + x` | `"v=3√2"` | `"v=4.242640687119286"` | exact_real_symbolic |
| tests/types/test_real_text.pairs.warp:10 | `sqrt(2) as string` | `"√2"` | `"1.4142135623730951"` | exact_real_symbolic |
| tests/types/test_shared_lists.pairs.warp:10 | `xs = [\"a\", \"b\"]; ys = xs; ys#2 = \"c\"; xs#2` | `'c'` | `"c"` | char_vs_text |
| tests/types/test_sized_array_values.pairs.warp:9 | `chars=[\"a\" \"b\"]; x=chars#2; x` | `'b'` | `"b"` | char_vs_text |
| tests/types/test_text_defaults.pairs.warp:9 | `class P{name:text=\"x\"}; P().name` | `'x'` | `"x"` | char_vs_text |
| tests/types/test_text_of_text.pairs.warp:4 | `text(text(5))` | `'5'` | `"5"` | char_vs_text |
| tests/types/test_text_of_text.pairs.warp:5 | `text text 5` | `'5'` | `class text class text 5` | prefix_word_call |
| tests/types/test_text_of_text.pairs.warp:6 | `text(str(5))` | `'5'` | `"5"` | char_vs_text |
| tests/types/test_text_of_text.pairs.warp:7 | `str(text(5))` | `'5'` | `"5"` | char_vs_text |
| tests/types/test_text_of_text.pairs.warp:8 | `text(5 as text)` | `'5'` | `"5"` | char_vs_text |
| tests/types/test_text_of_text.pairs.warp:9 | `(5 as text) as text` | `'5'` | `"5"` | char_vs_text |
| tests/types/test_type_calls.pairs.warp:11 | `b = byte 65; text(b)` | `"0x41"` | `"class byte 65"` | prefix_word_call |
| tests/types/test_type_calls.pairs.warp:15 | `int 3.7` | `3` | `class int 3.7` | prefix_word_call |
| tests/types/test_type_calls.pairs.warp:16 | `text 5` | `'5'` | `class text 5` | prefix_word_call |
| tests/types/test_type_of_lambda_variable.pairs.warp:4 | `f = x => x*2; type(f)` | `"function: (x) -> int"` | `class function` | type_of_function_signature |
| tests/types/test_type_of_lambda_variable.pairs.warp:5 | `f := x => x*2; type(f)` | `"function: (x) -> int"` | `class function` | type_of_function_signature |
| tests/types/test_type_of_lambda_variable.pairs.warp:6 | `f = x => x*2; type(function f)` | `"function: (x) -> int"` | `class function` | type_of_function_signature |
| tests/types/test_type_of_real_variable.pairs.warp:11 | `x = sqrt(2); x is float` | `0` | `yes` | exact_real_symbolic |
| tests/types/test_type_of_real_variable.pairs.warp:12 | `sqrt(2) is float` | `0` | `yes` | exact_real_symbolic |
| tests/types/test_type_static.pairs.warp:8 | `class P{x:int}; p = P(1); type(p)` | `'P'` | `class P` | type_values |
| tests/types/test_type_upgrading.pairs.warp:4 | `1.5+2.5` | `4` | `4` | num_int_vs_float |
| tests/types/test_type_upgrading.pairs.warp:5 | `10.0-3.0` | `7` | `7` | num_int_vs_float |
| tests/types/test_type_upgrading.pairs.warp:6 | `2.5*4.0` | `10` | `10` | num_int_vs_float |
| tests/types/test_type_values.pairs.warp:4 | `\"type is \" + type 3` | `"type is class int"` | `"type is class type 3"` | other |
| tests/types/test_type_values.pairs.warp:10 | `x = int; type x` | `"type"` | `class type class int` | prefix_word_call |
| tests/types/test_type_word_user_function.pairs.warp:8 | `double 4` | `4` | `class double 4` | prefix_word_call |
| tests/types/test_type_words.pairs.warp:4 | `double 2` | `2` | `class double 2` | prefix_word_call |
| tests/types/test_type_words.pairs.warp:5 | `long 2` | `2` | `class long 2` | prefix_word_call |
| tests/types/test_type_words.pairs.warp:6 | `float 2` | `2` | `class float 2` | prefix_word_call |
| tests/types/test_type_words.pairs.warp:7 | `int 2` | `2` | `class int 2` | prefix_word_call |
| tests/types/test_types.pairs.warp:12 | `2 as string` | `'2'` | `"2"` | char_vs_text |
| tests/types/test_types.pairs.warp:57 | `fun addier(a,b){b+a};addier(42.0,1.0)` | `43` | `43` | num_int_vs_float |
| tests/types/test_types.pairs.warp:60 | `fun addier(float a,float b){b+a};addier(42,1)+1` | `44` | `44` | num_int_vs_float |
| tests/types/test_unknown_conversion.pairs.warp:5 | `3 as text` | `'3'` | `"3"` | char_vs_text |
| tests/types/test_upcast_fields.warp:11 | `class Shape { name: text }; class Circle extends Shape { r: int }; s: Shape = Circle(\"a\", 2); s.name` | `'a'` | `"a"` | char_vs_text |
| tests/wasm/test_wasm.pairs.warp:4 | `x=9;⅓x` | `3` | `1/3 9` | other |
| tests/wasm/test_wasm.pairs.warp:31 | `fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)` | `5` | `1` | it_lambda_recursion |
| tests/wasm/test_wasm.pairs.warp:32 | `fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)` | `13` | `-11` | it_lambda_recursion |
| tests/wasm/test_wasm.pairs.warp:54 | `i=1.0;i` | `1` | `1` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:70 | `i=1.0;i=3;i=i/2;i=i*4` | `6` | `6` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:83 | `√9*-‖-3‖/3` | `-3` | `-3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:84 | `√9*‖-3‖/-3` | `-3` | `-3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:85 | `√9*-‖-3‖/-3` | `3` | `3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:129 | `-√9` | `-3` | `-3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:135 | `- √9` | `-3` | `-3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:143 | `√3^0` | `1` | `1` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:145 | `√3^2` | `3` | `2.9999999999999996` | exact_real_symbolic |
| tests/wasm/test_wasm.pairs.warp:252 | `true true and` | `yes` | `yes ø` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:253 | `false true and` | `no` | `no ø` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:254 | `false false and ` | `no` | `no 0` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:255 | `true false and ` | `no` | `yes 0` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:268 | `¬ 1` | `0` | `no 1` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:269 | `¬ 0` | `1` | `no 0` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:283 | `¬ (0 ⋁ 0 ∧ 1)` | `1` | `no 0` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:307 | `if(2){3}{4}` | `3` | `3 4` | logic_rpn_negation |
| tests/wasm/test_wasm.pairs.warp:312 | `fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)` | `5` | `1` | it_lambda_recursion |
| tests/wasm/test_wasm.pairs.warp:313 | `fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)` | `13` | `-11` | it_lambda_recursion |
| tests/wasm/test_wasm.pairs.warp:363 | `i=-9;√-i` | `3` | `3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:366 | `√100²` | `100` | `100` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:382 | `√9` | `3` | `3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:389 | `round 3.7` | `4` | `4` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:425 | `i=-9;√ -i` | `3` | `3` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:435 | `42.0/2.0` | `21` | `21` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:477 | `sum=0;for i in (1..3) {sum+=i};sum` | `6` | `3` | for_loop_iteration |
| tests/wasm/test_wasm.pairs.warp:504 | `√3**2` | `3` | `2.9999999999999996` | exact_real_symbolic |
| tests/wasm/test_wasm.pairs.warp:506 | `√3^2` | `3` | `2.9999999999999996` | exact_real_symbolic |
| tests/wasm/test_wasm.pairs.warp:511 | `√42.0*√42` | `42` | `42` | num_int_vs_float |
| tests/wasm/test_wasm.pairs.warp:512 | `√42*√42` | `42` | `42` | num_int_vs_float |
| tests/wasm/test_wasm.warp:64 | `sum=0\nfor i in (1..3) {sum+=i}\nsum` | `6` | `3` | for_loop_iteration |
| tests/web/test_web_playground.warp:6 | `fun h(){ return 2.5 }\nh() * 2` | `5` | `5` | num_int_vs_float |
| tests/welcoming/test_discarded_error_warning.pairs.warp:4 | `e = error \"?\"; e failed` | `yes` | `no` | other |
| tests/welcoming/test_footguns.pairs.warp:92 | `i=1;++i;i` | `2` | `1` | unary_sign_lists |
| tests/welcoming/test_footguns.pairs.warp:93 | `i=1;++i` | `2` | `"+1"` | unary_sign_lists |
| tests/welcoming/test_footguns.pairs.warp:158 | `i=3.5;.5+i` | `4` | `4` | num_int_vs_float |
| tests/welcoming/test_footguns.pairs.warp:383 | `fast a=0.1; fast b=0.2; fast c=0.3; (a+b)+c == a+(b+c)` | `no` | `yes` | other |
| tests/welcoming/test_footguns.pairs.warp:403 | `√2*√2 == 2` | `yes` | `no` | exact_real_symbolic |
| tests/welcoming/test_footguns.pairs.warp:404 | `√2*√2` | `2` | `2.0000000000000004` | exact_real_symbolic |
| tests/welcoming/test_footguns.pairs.warp:425 | `(sqrt(2)+sqrt(3))^2 == 5+2*√6` | `yes` | `no` | exact_real_symbolic |
| tests/welcoming/test_footguns.pairs.warp:436 | `f(x):=x+1; g(y):=y+1; f==g` | `yes` | `no` | function_equality |
| tests/welcoming/test_footguns.pairs.warp:437 | `f(x):=x+1; g(y):=y+1; f!=g` | `no` | `yes` | function_equality |
| tests/welcoming/test_footguns.pairs.warp:442 | `f(x):=(x+1)^2; g(x):=x^2+2*x+1; f==g` | `yes` | `no` | function_equality |
| tests/welcoming/test_footguns.pairs.warp:445 | `f(x,y):=(x - y)*(x+y); g(a,b):=a^2-b^2; f==g` | `yes` | `no` | function_equality |
| tests/welcoming/test_footguns.pairs.warp:446 | `f(x):=x/2+x/2; g(x):=x; f==g` | `yes` | `no` | function_equality |
| tests/welcoming/test_footguns.warp:41 | `fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fibo(m) = m < 2 ? m : fibo(m - 1) + fibo(m - 2); fib==fibo` | `yes` | `no` | other |
| tests/welcoming/test_footguns.warp:44 | `f(a:bool,b:bool):=not (a or b); g(a:bool,b:bool):=(not a) and (not b); f==g` | `yes` | `no` | function_equality |
| tests/welcoming/test_variable_applied.pairs.warp:6 | `add(a) := b => a + b; add(2)(5)` | `7` | `closure_lambda_1 5` | fn_value_not_applied |
| tests/welcoming/test_welcoming_elements.pairs.warp:6 | `names = [\"A\", \"B\"]; p = names[1]; p` | `'B'` | `"B"` | char_vs_text |
| tests/welcoming/test_welcoming_elements.pairs.warp:7 | `names = ['A', 'B']; p = names[1]; p` | `'B'` | `"B"` | char_vs_text |
| tests/welcoming/test_welcoming_elements.pairs.warp:8 | `names = \"ABCDEF\"; p = names[1]; p` | `'B'` | `"B"` | char_vs_text |
| tests/welcoming/test_welcoming_globals.pairs.warp:35 | `x=10; x=floor(x/2)` | `5` | `5` | num_int_vs_float |
| tests/welcoming/test_welcoming_globals.pairs.warp:36 | `x=10; x=floor(x/2)+1` | `6` | `6` | num_int_vs_float |
| tests/welcoming/test_welcoming_globals.pairs.warp:37 | `x=10; while x>1 { x=floor(x/2) }; x` | `1` | `1` | num_int_vs_float |
| tests/welcoming/test_welcoming_maps.pairs.warp:45 | `m = {A: 5}; s = \"\"; for k in m { s = s + k }; s` | `'A'` | `"A"` | char_vs_text |
| tests/welcoming/test_welcoming_maps.warp:11 | `fun f(g) { c = \"\"; for e in g[\"A\"] { c = c + e[0] }; c }; f({A: [[\"B\", 4]]})` | `'B'` | `"B"` | char_vs_text |
| tests/welcoming/test_welcoming_print.pairs.warp:25 | `import floor from \"m\"; x=10.0; x=floor(2.5); x+1` | `3` | `3` | num_int_vs_float |
