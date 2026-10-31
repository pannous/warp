# Full suite 2026-10-31 at 302b5481cd: regressions since aa60d3ddc3 (10-29)

3488 of 11643 failed (was 4149 of 11173). Compared by file and code: 800 lines went green, 235 went red.

Raw list: data/full-suite/regressed.txt in the Integrator's checkout. Groups, largest first. Some are decided changes the tests still pin (`a -1` is a list by spacing, `5!` the factorial: decisions canonical-output, factorial-bang); those are tests to update, not bugs:

## 128 × float where int (use sound / reals.warp:39 real_of)
- `test/control/test_task_globals.pairs.warp:8 "use sound; job = go { C4 * 100 }; await job" 26163: got Error("outside the core: a float where a int is needed at /Users/me/dev/bin/warp-builds/native-48`
- `test/functions/test_arity_and_type_overloads.warp:16 "use sound\nclass Box{v}\nplay(b: Box) := 7\nplay(Box(1))" 7: got Error("outside the core: a float where a int is needed at 324:29 (no owner)")`
- `test/functions/test_function_typed_overloads.pairs.warp:7 "use sound; x = C4; x" 261.63: got Error("outside the core: a float where a int is needed at /Users/me/dev/bin/warp-builds/native-485e3f51a/`
- `test/functions/test_host_long_lists.pairs.warp:3 "use sound; #effected([0.5 for i in 1 to 44100], [\"gain\", 1.0])" 44100: got Error("outside the core: a float where a int is needed at 14:99 (no own`

## 8 × N
- `test/control/test_call_condition_block.pairs.warp:4 "profile = \"Cargo.toml\"; if exists profile {1} else {2}" 1: got 2`
- `test/control/test_match_item_patterns.pairs.warp:11 "match [[1, \"x\"], [1, \"y\"]] { [[1, s]...] => count(s); _ => 0 }" 2: got 0`
- `test/control/test_run_time_blocks.pairs.warp:29 "5!" 5: got 120`
- `test/functions/test_nonlocal_lambdas.pairs.warp:8 "def f(){ n=1; g = () => { n = 5; n }; g() + n }; f()" 6: got 10`

## 7 × Error("outside the core: the call i(…) at N:N (no 
- `test/types/test_declared_int_of_float_expression.pairs.warp:4 "int i=2.0*3; i" 6: got Error("outside the core: the call i(…) at 1:1 (no owner)")`
- `test/types/test_declared_int_of_float_expression.pairs.warp:5 "int i=2*3; i" 6: got Error("outside the core: the call i(…) at 1:1 (no owner)")`
- `test/types/test_declared_int_of_float_expression.pairs.warp:9 "int i=3.5*2; i" 7: got Error("outside the core: the call i(…) at 1:1 (no owner)")`
- `test/types/test_declared_int_of_float_expression.pairs.warp:10 "int i=0.1+0.9; i" 1: got Error("outside the core: the call i(…) at 1:1 (no owner)")`

## 5 × [N, -N]
- `test/lists/test_list_literal_sign.pairs.warp:15 "a=5; a -1" 4: got [5, -1]`
- `test/lists/test_signed_operand_list.pairs.warp:14 "x=5;x -1" 4: got [5, -1]`
- `test/operators/test_minus_after_callable.pairs.warp:19 "x=5; x -2" 3: got [5, -2]`
- `test/operators/test_minus_after_callable.pairs.warp:21 "int = 3; int -1" 2: got [3, -1]`

## 4 × Error("outside the core: an assignment to a type a
- `test/types/test_union_aliases.pairs.warp:2 "type View = list or int; f(v: View) := 1; f(3) + f([1])" 2: got Error("outside the core: an assignment to a type at 1:13 (no owner)")`
- `test/types/test_union_aliases.pairs.warp:3 "type View = list or int; class C{v: View}; C(3).v" 3: got Error("outside the core: an assignment to a type at 1:13 (no owner)")`
- `test/types/test_union_aliases.pairs.warp:4 "type View = list or int; x: View = [1]; x" [1]: got Error("outside the core: an assignment to a type at 1:13 (no owner)")`
- `test/types/test_union_aliases.pairs.warp:7 "type V = list or int; f(x: int): V := if x > 0 then x else ø; [f(0), f(2)]" [ø, 2]: got Error("outside the core: an assignment to a type at 1:10 (no owner`

## 3 × Error("outside the core: an assignment to `… : …` 
- `test/control/test_or_error_type.pairs.warp:7 "x: int or error = 3; x" 3: got Error("outside the core: an assignment to '… : …' at 1:19 (no owner)")`
- `test/control/test_or_error_type.pairs.warp:8 "x: int or error = error(\"e\"); if x failed then 1 else 2" 1: got Error("outside the core: an assignment to '… : …' at 1:25 (no owner)")`
- `test/control/test_or_error_type.pairs.warp:18 "x: symbol or int = 3; x" 3: got Error("outside the core: an assignment to '… : …' at 1:20 (no owner)")`

## 3 × nat
- `test/ffi/test_builtin_lines.pairs.warp:5 "type(floor(2.5))" "int": got nat`
- `test/functions/test_bare_host_words.pairs.warp:51 "use prelude; type random_int()" "int": got nat`
- `test/numbers/test_rational_type.pairs.warp:31 "type(2.0)" "int": got nat`

## 3 × Error("`square N < N` reads two ways: square(N < 
- `test/functions/test_abs_square_comparison.pairs.warp:4 "use math; abs -3 < 5 and square 3 < 10" yes: got Error("'square 3 < 10' reads two ways: square(3 < 10) or (square 3) < 10; square's parameter `
- `test/functions/test_abs_square_comparison.pairs.warp:5 "use math; abs -3 < 2 and square 3 < 10" no: got Error("'square 3 < 10' reads two ways: square(3 < 10) or (square 3) < 10; square's parameter i`
- `test/functions/test_abs_square_comparison.pairs.warp:8 "square(x):=x*x; abs -3 < 5 and square 3 < 10" yes: got Error("'square 3 < 10' reads two ways: square(3 < 10) or (square 3) < 10; square's para`

## 3 × Error("outside the core: the call total(…) at N:N
- `test/functions/test_variadic.pairs.warp:4 "def total(xs...){sum xs}; total(1,2,3)" 6: got Error("outside the core: the call total(…) at 1:33 (no owner)")`
- `test/functions/test_variadic.pairs.warp:19 "def total(xs...){sum xs}; ys=[1,2,3]; total(...ys)" 6: got Error("outside the core: the call total(…) at 1:45 (no owner)")`
- `test/functions/test_variadic.pairs.warp:21 "def total(xs...){sum xs}; ys=[1,2,3]; total(*ys)" 6: got Error("outside the core: the call total(…) at 1:45 (no owner)")`

## 3 × Error("not a map; keys, values and entries are a m
- `test/modules/test_node_json.pairs.warp:15 "parse(\"a * (b + 1)\").json" "{\"*\":[\"a\",{\"+\":[\"b\",1]}]}": got Error("not a map; keys, values and entries are a map's; a list's positions are 1..cou`
- `test/modules/test_node_json.pairs.warp:17 "parse(\"{a: 1+2}\").json" "{\"a\":{\"+\":[1,2]}}": got Error("not a map; keys, values and entries are a map's; a list's positions are 1..count(xs) at 170:6`
- `test/modules/test_node_json.pairs.warp:21 "parse(\"f(x) := x*2\").json" "{\"(f x)\":{\"*\":[\"x\",2]}}": got Error("not a map; keys, values and entries are a map's; a list's positions are 1..count(x`

## 3 × Error("not a character at N:N")
- `test/types/test_hex_int.pairs.warp:12 "to_base(255, 16, upper: true)" "FF": got Error("not a character at 265:137")`
- `test/types/test_hex_int.pairs.warp:13 "to_base(255, 16)" "ff": got Error("not a character at 265:137")`
- `test/welcoming/test_golf_words.pairs.warp:25 "use math; to_base(5, 2).count_of(\"1\")" 2: got Error("not a character at 265:137")`

## 3 × ø
- `test/types/test_optional_list_items.pairs.warp:5 "xs = [1, ø]; type(xs#2)" "empty": got ø`
- `test/types/test_type_static.pairs.warp:6 "type(ø)" "empty": got ø`
- `test/types/test_type_static.pairs.warp:7 "x = ø; type(x)" "empty": got ø`

## 2 × "T{n: 'x'}"
- `test/control/test_if_instance_branches.pairs.warp:4 "class T{n:text}; a = if 1>2 then ø else T{n:\"x\"}; str(a)" "T{n:'x'}": got "T{n: 'x'}"`
- `test/control/test_if_instance_branches.warp:7 "class T{n:text}; a = if 1>2 then T{n:\"y\"} else T{n:\"x\"}; str(a)" "T{n:'x'}": got "T{n: 'x'}"`

## 2 × Error("`(f (sqrt N))` fits no single variant of f(
- `test/functions/test_computed_argument_kinds.pairs.warp:3 "f(x: int) := \"int\"; f(x: float) := \"float\"; f(sqrt(2))" "float": got Error("'(f (sqrt 2))' fits no single variant of f(int), f(float) at`
- `test/functions/test_computed_argument_kinds.pairs.warp:4 "f(x: int) := \"int\"; f(x: number) := \"number\"; f(sqrt(2))" "number": got Error("'(f (sqrt 2))' fits no single variant of f(int), f(number`

## 2 × Error("write test/modules/std_file_written.txt fai
- `test/modules/test_std_file.pairs.warp:14 "use file; write(\"test/modules/std_file_written.txt\", \"hi\"); read(\"test/modules/std_file_written.txt\")" "hi": got Error("write test/modules/std_file_wr`
- `test/modules/test_std_file.pairs.warp:15 "write(\"test/modules/std_file_written.txt\", \"hi\"); read(\"test/modules/std_file_written.txt\")" "hi": got Error("write test/modules/std_file_written.txt `

## 2 × "view·opaque: [\"type\", [\"symbol\", 'P'], [\"lis
- `test/node/test_parsed_node_walks.pairs.warp:31 "n = parse(\"class P { x: int }\"); \"\\(n)\"" "type P {x:type int ø}": got "view·opaque: [\"type\", [\"symbol\", 'P'], [\"list\", '{', ' ', [\"key\", `
- `test/node/test_parsed_node_walks.pairs.warp:32 "t = \"class P { x: int }\"; n = parse(t); \"\\(n)\"" "type P {x:type int ø}": got "view·opaque: [\"type\", [\"symbol\", 'P'], [\"list\", '{', ' ', [\"`

## 2 × Error("outside the core: the call it*it(…) at N:N
- `test/operators/test_operator_declarations.pairs.warp:9 "prefix operator ∆ := it*it; ∆3" 9: got Error("outside the core: the call it*it(…) at 1:30 (no owner)")`
- `test/operators/test_operator_declarations.pairs.warp:10 "prefix operator ∆ := it*it; ∆3+1" 10: got Error("outside the core: the call it*it(…) at 1:30 (no owner)")`

## 2 × Error("outside the core: the call it*-N(…) at N:N
- `test/operators/test_superscript_operator_declarations.pairs.warp:10 "prefix operator ⁻ := it*-1; ⁻3" -3: got Error("outside the core: the call it*-1(…) at 1:30 (no owner)")`
- `test/operators/test_superscript_operator_declarations.pairs.warp:11 "prefix operator ⁻ := it*-1; ⁻3+5" 2: got Error("outside the core: the call it*-1(…) at 1:30 (no owner)")`

## 2 × Error("outside the core: a parameter of kind it·N-
- `test/types/test_union_spellings.pairs.warp:7 "f(a: int or float) := a; f(2.5)" 2.5: got Error("outside the core: a parameter of kind it·5->(float (int it·5)) at 1:28 (no owner)")`
- `test/types/test_union_types.pairs.warp:4 "f(n: int or float) := n * 2; [f(2), f(1.5)]" [4, 3]: got Error("outside the core: a parameter of kind it·5->(float (int it·5)) at 1:33 (no owner)")`

## 1 × Error("outside the core: running a node as code (`
- `test/control/test_control_words.pairs.warp:33 "f:={1+2}; f!" 3: got Error("outside the core: running a node as code ('x!') at 1:7 (no owner)")`

## 1 × Error("outside the core: a node for a typed field 
- `test/control/test_error_not_in_data.warp:6 "e = try 10/0; class pt{x:int or error}; p = pt(e); if p.x failed then 1 else 2" 1: got Error("outside the core: a node for a typed field of pt at 1:48 (no`

## 1 × Error("not an int at N:N")
- `test/control/test_for_forms.pairs.warp:21 "n=0; for 1..3 {n+=it}; s=\"\"; for \"ab\" {s+=it}; s" "ab": got Error("not an int at 14:116")`

## 1 × Error("outside the core: the unknown name gcd: `us
- `test/control/test_function_objects.warp:72 "use math\n(function gcd) is function" yes: got Error("outside the core: the unknown name gcd: 'use math' defines it at 2:2 (no owner)")`

## 1 × Error("m.description is no word of a function: did
- `test/control/test_function_objects.warp:122 "m = {description: 3}\nm.description" 3: got Error("m.description is no word of a function: did you mean m.doc?")`

## 1 × "{n: 'x'}"
- `test/control/test_if_instance_branches.pairs.warp:7 "a = if 1>2 then {n:\"y\"} else {n:\"x\"}; str(a)" "{n:'x'}": got "{n: 'x'}"`

## 1 × "[T{n: 'x'}]"
- `test/control/test_if_instance_branches.warp:4 "class T{n:text}; a = [(if 1>2 then ø else T{n:r}) for r in [\"x\"]]; str(a)" "[T{n:'x'}]": got "[T{n: 'x'}]"`

## 1 × Error("")
- `test/control/test_system_signals.pairs.warp:5 "def stop(){ exit(3) }; stop(); 7" ø: got Error("")`

## 1 × Error("i is a main-level variable: declare it `glo
- `test/control/test_task_name_scope.pairs.warp:4 "f(k) := { n = 0; for i in 1..k { n += i }; n }; i = 7; f(3) + i" 10: got Error("i is a main-level variable: declare it 'global i' to change it from a `

## 1 × Error("wasm trap: call stack exhausted at N:N")
- `test/control/test_try_stack_overflow.warp:8 "global calls = 0; down(n) := { calls = calls + 1; if n == 0 then 0 else 1 + down(n - 1) }; r = try down(1000000) else 0; calls > 1000" 1: got Error("wasm`

## 1 × Error("text of node at N:N")
- `test/ffi/test_host_words.pairs.warp:12 "x = read(\"Cargo.toml\"); print \"read\"; count(x) > 3" yes: got Error("text of node at 1:1")`

## 1 × "b"
- `test/functions/test_arity_overloads.warp:55 "class Point { x: int; name: text }\nPoint.label := it.name + \"!\"\nPoint.label(v) := it.name = v\ncfg = {label: 3}\np = Point(1, \"a\")\np.label = \"b\"`

## 1 × "int"
- `test/functions/test_computed_argument_kinds.pairs.warp:6 "f(x: int) := \"int\"; f(x: float) := \"float\"; f(-sqrt(9))" "float": got "int"`

## 1 × "{li: {color: \"blue\"}}"
- `test/functions/test_data_parameter.warp:21 "theme = 'blue'\nwritten(code: data) := str(code)\nwritten{ li{ color: theme } }" "{li:{color:\"blue\"}}": got "{li: {color: \"blue\"}}"`

## 1 × Error("not a function at N:N")
- `test/functions/test_expression_calls.pairs.warp:9 "a = 1; b = 2; c = 3; (a + b)(c)" 9: got Error("not a function at 1:23")`

## 1 × Error("outside the core: the unknown name twice: `
- `test/functions/test_nested_prefix_calls.pairs.warp:11 "twice(f, x) := f(f(x)); square(n) := n*n; twice square 3" 81: got Error("outside the core: the unknown name twice: 'use prelude' defines it at `

## 1 × Error("outside the core: the call b(…) at N:N (no 
- `test/functions/test_preposition_calls.warp:49 "f(a, b) := a - b\nf b=1 a=5" 4: got Error("outside the core: the call b(…) at 2:1 (no owner)")`

## 1 × "[N, N]"
- `test/functions/test_runtime_function_equality.warp:7 "sample(x) := x; g(x) := x + 1; f(samples) := [g(sample) for sample in samples]; string(f([5, 6]))" "[6 7]": got "[6, 7]"`

## 1 ×  N (sqrt N)
- `test/functions/test_runtime_kind_arithmetic.pairs.warp:4 "def f(m){ m.a * m.b }; f({a:sqrt(2), b:2})" 2.8284271247461903: got  2 (sqrt 2)`

## 1 ×  the host word not sound.queued is  in warp yet
- `test/functions/test_sequence_values.warp:12 "f() := {\n1\nstd_io(\"sound\", \"queued\", [])\n}; f()" 0: got  the host word not sound.queued is  in warp yet`

## 1 × Error("outside the core: the call count(…) at N:N
- `test/functions/test_variadic.pairs.warp:7 "def count(xs...){#xs}; count(1,2,3,4)" 4: got Error("outside the core: the call count(…) at 1:30 (no owner)")`

## 1 × Error("outside the core: the unknown name total (n
- `test/functions/test_variadic.pairs.warp:13 "def total(xs...){sum xs}; total()" 0: got Error("outside the core: the unknown name total (no owner)")`

## 1 × Error("outside the core: the unknown name total at
- `test/functions/test_variadic.pairs.warp:16 "total(xs...) := sum xs; total 1 2 3" 6: got Error("outside the core: the unknown name total at 1:25 (no owner)")`

## 1 × Error("outside the core: the operator to with noth
- `test/functions/test_variadic.pairs.warp:20 "def total(xs...){sum xs}; ys=[1,2,3]; total(ys...)" 6: got Error("outside the core: the operator to with nothing after it at 1:45 (no owner)")`

## 1 × Error("outside the core: a function value in # at 
- `test/lists/test_count_after_function.pairs.warp:4 "f(n) := n * 2; f #[1 2 3]" 6: got Error("outside the core: a function value in # at 1:16 (no owner)")`

## 1 × [[N, -N], N]
- `test/lists/test_list_literal_sign.pairs.warp:12 "a=5; [a -1, 2]" [4, 2]: got [[5, -1], 2]`

## 1 × [[N, -N]]
- `test/lists/test_list_literal_sign.pairs.warp:14 "a=5; [(a -1)]" [4]: got [[5, -1]]`

## 1 × [N, N]
- `test/modules/test_std_random.pairs.warp:10 "use random; seed(42); [random_below(100), random_below(100)]" [15, 64]: got [74, 34]`

## 1 × Error("int of node at N:N")
- `test/numbers/test_float_exact_context.pairs.warp:22 "x: int|rational = 1/3; x * 3" 1: got Error("int of node at 1:24")`

## 1 × Error("outside the core: a node as rational at /Us
- `test/numbers/test_nearest.pairs.warp:16 "sqrt(2) as rational == 14142135623730951/10000000000000000" yes: got Error("outside the core: a node as rational at /Users/me/dev/bin/warp-builds/native-485e`

## 1 × -N
- `test/operators/test_minus_after_callable.pairs.warp:11 "f(x):=x*2; f -2 + 1" -3: got -2`

## 1 × Error("outside the core: the call size(…) at N:N 
- `test/operators/test_minus_after_callable.pairs.warp:22 "p = {size: 5}; p.size -1" 4: got Error("outside the core: the call size(…) at 1:18 (no owner)")`

## 1 × Error("`sum [N N] == N` reads two ways: sum([N N] 
- `test/parser/test_braceless_range_argument.pairs.warp:8 "y = sum [1 2] == 3; y" yes: got Error("'sum [1 2] == 3' reads two ways: sum([1 2] == 3) or (sum [1 2]) == 3; sum's parameter is untyped: write`

## 1 × "Point{x: N}"
- `test/parser/test_comments.warp:8 "# Rust's\nclass Point { x: int }\n# it's\n/# don't #/\nstr(Point{x: 1})" "Point{x:1}": got "Point{x: 1}"`

## 1 × Error("outside the core: a text given to a variabl
- `test/text/test_character_steps.pairs.warp:6 "c = 'b'; s = ''; for x in c to 'd' { s += x }; s" "bcd": got Error("outside the core: a text given to a variable declared int at 1:27 (no owner)")`

## 1 × "a ${x} b"
- `test/text/test_interpolation.pairs.warp:27 "'a ${x} b'" "a ${x} b": got "a ${x} b"`

## 1 × "a${b}"
- `test/text/test_interpolation.pairs.warp:44 "\"a\\$(b)\"" "a$(b)": got "a${b}"`

## 1 × "[[N, N], [N, N], [N, N]]"
- `test/text/test_nested_list_text.warp:4 "transpose(m) := { tt = []; for c in 1..#m#1+1 { row = []; for r in 1..#m+1 { row.add(m#r#c) }; tt.add(row) }; tt }; str(transpose([[1 2 3] [4 5 6]]))" "[[1 4]`

## 1 × "Point{x: N, name: 'a'}"
- `test/text/test_quoted_container_texts.warp:4 "class Point{x:int name:text}; p = Point{x:1 name:\"a\"}; string(p)" "Point{x:1 name:'a'}": got "Point{x: 1, name: 'a'}"`

## 1 × "at Point{x: N, y: N}"
- `test/types/test_class_cases_ported.warp:49 "class Point{x:int; y:int}; p = Point(1, 2); \"at \\(p)\"" "at Point{x:1 y:2}": got "at Point{x: 1, y: 2}"`

## 1 × "[\"hi\", 'a']"
- `test/types/test_declared_list_marks.pairs.warp:11 "names: texts = [\"hi\"]; ys = names; ys.add(\"a\"); string(names)" "[\"hi\" 'a']": got "[\"hi\", 'a']"`

## 1 × Error("outside the core: the call f(…) at N:N (no
- `test/types/test_one_dispatch.pairs.warp:67 "f(xs...) := #xs; 3.f(4, 5)" 3: got Error("outside the core: the call f(…) at 1:18 (no owner)")`

## 1 × Error("wasm trap: cast failure at N:N")
- `test/types/test_partial_time_comparisons.pairs.warp:23 "time == now" no: got Error("wasm trap: cast failure at 13:2")`

## 1 × "Point{x: N, y: N}"
- `test/types/test_struct_instances.warp:13 "class Point{x:int; y:int}; p = Point(3, 4); i=0; while i<3 { p.x += i; i++ }; string(p)" "Point{x:6 y:4}": got "Point{x: 6, y: 4}"`

## 1 × Error("a GPU list [fN] takes only #, xs#i, xs#i =
- `test/types/test_typed_arrays.pairs.warp:21 "if 0 { x = int[3] }; x" ø: got Error("a GPU list [f32] takes only #, xs#i, xs#i = number, for, print and the GPU's words: copy it to a list with [x for x `

## 1 × Error("fixtures/alias_a.warp: no such file to use"
- `test/types/test_union_aliases.pairs.warp:10 "use \"../fixtures/alias_a.warp\"; use \"../fixtures/alias_b.warp\"; f(v: Spot) := 1; f(3)" 1: got Error("fixtures/alias_a.warp: no such file to use")`

## 1 × "{a: N, b: N}"
- `test/welcoming/test_keys_of_a_map_parameter.warp:5 "merge(a, b) := { out = a; for k in keys(b) { out[k] = b[k] }; out }; string(merge({a: 1}, {b: 2}))" "{a:1 b:2}": got "{a: 1, b: 2}"`

## 1 × [ø, ø]
- `test/welcoming/test_welcoming_print.pairs.warp:9 "xs=[1,2]; print(xs)" ø: got [ø, ø]`
