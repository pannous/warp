//! Warp's type checks against W0, the Lean model of warp's type theory (notes/type_theory.md, lean/WarpTypes):
//! the proofs must build without `sorry`, and warp must reject every program the model rejects, except the known
//! holes, each with its card. A fixed hole fails the test until it is taken off the list.
use warp::law::type_model::{admits_disagreements, axioms, export, model_sources, outcomes, verdicts, warp_value, warp_verdict, ModelVerdict};

/// The soundness theorems the tie-in rests on
const THEOREMS: [&str; 7] = ["Warp.progress", "Warp.preservation", "Warp.safety", "Warp.typeOf_sound", "Warp.check_safe", "Warp.step_sound", "Warp.run_sound"];
/// The axioms of Lean's core logic; anything else (sorryAx above all) is an unproved assumption
const STANDARD_AXIOMS: [&str; 3] = ["propext", "Quot.sound", "Classical.choice"];
/// An unproved step: the `sorry` tactic, or a declared `axiom`
const SORRY: &str = "sorry";
const AXIOM_DECLARATION: &str = "axiom ";

/// Programs both sides judge alike
const CORPUS: &[&str] = &[
	"1 + 2",
	// a conversion to bool is a bool (card bool-conversion)
	"2 as bool",
	"def f(x) -> bool { x }; f(2)",
	// a conversion has its type's kind (card kind-name)
	"x = 3.7 as int; x = \"a\"",
	"1 + 2.5",
	"true + 1",
	"\"a\" + 1",
	"1 < 2",
	"x = 1; x = 2",
	"x = 1; x = 2.5",
	"x = 1; x = \"a\"",
	"x = [1]; x = 5",
	"x: int = 1.5",
	"x: int = 1; x = 2.5",
	"x: float = 1",
	"x: int = true",
	"x: bool = 1",
	"x: bool = 0; x = 1",
	"f(b: bool) := b; f(1)",
	"x: bool = 2",
	"f(b: bool) := b; f(2)",
	"b = true; b = 2",
	"x: text = 3",
	"x: text = \"ab\"; x = 3",
	"const c = 1; c = 2",
	"z := 1; z = 2",
	"y = 3; z := y*y; y = 4; z",
	"xs = [1, 2]; xs#1",
	"xs = [1, 2]; xs#5",
	"xs = [1, 2]; try xs#5 catch 0",
	"xs = [1]; xs.add(\"a\")",
	"xs = [1]; xs = [\"a\"]",
	"xs = []; xs.add(1)",
	"xs: ints = [1]; xs.add(2)",
	"xs: ints = [1]; xs.add(\"a\")",
	"xs: ints = [1]; xs = [\"a\"]",
	"xs: texts = [420]",
	"xs: ints = [1]; xs = xs + [2]",
	"xs: ints = [1, 2]; xs#1 + 1",
	"f(x: int) := x + 1; f(2)",
	"f(x: int) := x + 1; f(1.5)",
	"f(x: int) := x + 1; f(\"ab\")",
	"f(x: int) := x + 1; f([1, 2])",
	"f(x: float) := x * 2; f(1)",
	"f(n: int) := if n < 1 then 1 else n * f(n - 1); f(5)",
	"f(x) := x; f(3)",
	"f(x: text) := x; f(3)",
	"f(x: int) := x + 1; y = f(2); y = \"a\"",
	"i = 0; while i < 3 do i = i + 1; i",
	"if 1 < 2 then 1 else \"a\"",
	"try error(\"no\") catch 1",
	"x = 1; x == \"a\"",
	"3 - 1",
	"\"a\" - 1",
	"2 * 2.5",
	"f(a, b) := a - b; f(5, 1)",
	"f(a: int, b: text) := b + a; f(1, \"x\")",
	"f(a: int, b: text) := a; f(\"x\", 1)",
	"f(a, b) := a - b; f(1)",
	"def f(x){ x + 1 }; f(2)",
	"def two(){2}; two() + 1",
	"f(xs, n) := { if n == 0 then 1 else f(xs, n - 1) }; f([1,2], 2)",
	"f(x) := x; f(3); f(\"a\")",
	"f(x) := x * 2; f(3); f(2.5)",
	"f(x) := x + 1; f(3); f(\"a\")",
	"f(x) := x - 1; f(3); f(\"a\")",
	"x: any = 2; x * 3",
	"x: any = 2; x < 3",
	"if(2*1){3*1} else {4*1}",
	"s = 0; s += 2; s -= 1; s",
	"f(xs) := xs + [1]; f([2])",
	"f(xs) := xs#1; f([2, 3])",
	"\"hi\"#1",
	"x = \"abcde\"; x#4",
	"x: int | text = 3; x",
	"x: int | text = 3; x = \"ab\"; x",
	"x: (int|text) = 3; x = \"ab\"; x",
	"x: int or text = \"ab\"; x = 4; x + 1",
	"x: float | text = 2; x",
	"x: int | text = 3; x = 2.5",
	"x: int | text = [1, 2]",
	"x: int | float = 3; x = \"ab\"",
	"x: int | text = 3; x * 2",
	"f(x: int | text) := x; f(\"ab\")",
	"f(x: int or text) := x; f(3)",
	"f(x: int | text) := x; f(2.5)",
	"x: int | ø = 3; x",
	"x: int | ø = 3; x = \"a\"",
	"class P { x }; p = P(1); p.x + 1",
	"b = true; b = 1",
	"class Point { x: int; y: int }; p = Point(1, 2); p.x",
	"class Point { x: int; y: int }; p = Point(1, 2); p.x = 5; p.x",
	"class Point { x: int; y: int }; p = Point(1, 2); p.x = \"a\"",
	"class Point { x: int }; p = Point(\"a\")",
	"class Point { x: int }; f(q: Point) := q.x = 7; p = Point(1); f(p); p.x",
	"class P { x: int }; p = P(1); q = p; q.x = 7; p.x",
	"class P { x: int }; p = P(1); xs = [p]; p.x = 5; xs#1.x",
	"class P { x: int }; p = P(1); g(q: P) := q; r = g(p); r.x = 9; p.x",
	"class Shape { name: text }; class Circle extends Shape { r: int }; c = Circle(\"a\", 2); c.r",
	"class Shape { name: text }; class Circle extends Shape { r: int }; s: Shape = Circle(\"a\", 2); s.name",
	"class Shape { name: text }; class Circle extends Shape { r: int }; s = Circle(\"a\", 2); s.r",
	"type Color = red | rgb(r: int, g: int, b: int); c = rgb(1, 2, 3); c.r",
	"type Color = red | rgb(r: int, g: int, b: int); c = rgb(1, 2, 3); c is Color",
	"type Color = red | rgb(r: int, g: int, b: int); c = rgb(1, \"a\", 3)",
	"type Shape = circle(r: int) | square(s: int); f(x: Shape) := 1; f(circle(2))",
	"type Shape = circle(r: int) | square(s: int); f(x: circle) := x.r; f(square(2))",
	"on ask { 42 } in { emit ask }",
	"compute() := (emit ask) * 2; on ask { 21 } in { compute() }",
	"compute() := emit ask{n: 4}; on ask { event.n * 10 } in { compute() }",
	"compute() := emit ask; on ask { 1 } in { on ask { 2 } in { compute() } }",
	"a = 0; compute() := emit ask; on ask { 1 } in { a = on ask { 2 } in { compute() }; a * 10 + compute() }",
	"on ask { 7 }; compute() := emit ask; a = on ask { 2 } in { compute() }; a * 10 + compute()",
	"y = 0; on ask { y = 5; y + 10 } in { emit ask }",
	"y = 0; on ask { 1 } in { on ask { y = emit ask; y + 10 } in { emit ask } }",
	"on fail { break 0 } in { emit fail; 5 }",
	"n = 0; on fail { break 1 } in { n += 10; emit fail; n += 100 }; n",
	"on fail { break 1 } in { on stop { break 2 } in { emit fail; 9 } }",
	"on fail { break 3 } in { try { emit fail; 4 } else 5 }",
	"on ask { 3 } in { on fail { break 0 } in { emit ask * 2 } }",
	"on fail { break } in { emit fail; 5 }",
	"on ask { 2 }; 3 * emit ask + 1",
	"global n = 0; def f(x) { n = 5; x }; f(3); n",
	"y = 3; def z() { global y; y * y }; y = 4; z()",
	"global n = 0; def f() { n += 1 }; f(); f(); n",
	"global x; x = 7; x + 1",
	"global k = 7",
	"1 != 2",
	"class P { x: int }; p = P(1); q = P(1); p == q",
	"class P { x: int }; p = P(1); q = P(2); p == q",
	"class P { x: int }; class Q { x: int }; p = P(1); q = Q(1); p == q",
	"class P { x: int }; [P(1)] == [P(1)]",
	"class P { x: int }; p = P(1); q = p; p === q",
	"class P { x: int }; p = P(1); q = P(1); p === q",
	"class P { x: int }; p = P(1); q = p; p same q",
	"class P { x: int }; p = P(1); q = P(1); p same as q",
	"class P { x: int }; p = P(1); q = p; p is the same as q",
	"class P { x: int }; p = P(1); q = P(1); p !== q",
	"0 === false",
	"n = 0; on alarm { n += 1 }; emit alarm; emit alarm; n",
	"level = 0; on alarm { level = event.level }; emit alarm{level: 3}; level",
	"def check(x) { if x > 2 { emit too big{value: x} }; x }; check(1)",
	"y: any = 3; x: int = y; x",
	"f(n: int) := n + 1; y: any = 3; f(y)",
	"x: int = 0; xs = [1, \"a\"]; x = xs#1; x",
	"f(n: int) := n; f(\"a\")",

	"class Shape { name: text }; class Circle extends Shape { r: int }; s: Shape = Circle(\"a\", 2); s.r",
	"type Color = red | rgb(r: int, g: int, b: int); c: Color = rgb(1, 2, 3); c.r",

	"s = 0; for x in [1, 2, 3] { s += x }; s",
	"out = []; for x in [1, 2] { out.add(x) }; out",
	"class P { x: int }; s = 0; for p in [P(1), P(2)] { s += p.x }; s",
	"t = \"\"; for w in [\"a\", \"b\"] { t = t + w }; t",

	"def f(){ m = 5; m }; f()",
	"def f(){ m = 5; m = \"a\"; m }; f()",
	"product(xs) := { out = 1; for x in xs { out = out * x }; out }; product([2, 3, 4])",
	"unique(xs) := { out = []; for x in xs { out.add(x) }; out }; unique([1, 2])",
	"n = 0; def f(x) { n = 5; x }; f(3); n",
	"fact(n) := { r = 1; if n > 1 then r = n * fact(n - 1); r }; fact(5)",
	"g(n) := n; fact(n) := { r = 1; r = g(n); r }; fact(5)",
	"y: any = 3; class P { v: int }; p = P(1); p.v = y; p.v",

	"let x = 1; x = 2; x",
	"shared n = 5; n += 2; n",
	"String s = 'ab'; s",
	"String s = 3; s",
	"int i = 2; i = \"a\"; i",
	"int i = 2.5; i",

	"0..3",
	"1 to 3",
	"s = 0; for i in 0..3 { s += i }; s",
	"s = 0; for i in 1 to 3 { s += i }; s",
	"3..1",
	"count(n) := { c=0; for i in 0..n { c += 1 }; c }; count(5)",
	"to add number a to number b: a+b; add 1 to 2",
	"b = 0; a = on fail { break 1 } in { b = on fail { break 2 } in { emit fail; 9 }; b * 10 }; a",

	"0 and 7",
	"1 and 7",
	"(not 0) and 7",
	"not 1==2 and 2==2",
	"i=0; !i",
	"f(a:bool,b:bool):=not (a or b); f(true, false)",
	"x = 1 < 2 or 3 < 2; x",

	"s = 0; for x in 3 { s += x }; s",

	"xs = [1, 2]; a, b = xs; a*10+b",
	"f() := [1, 2]; a, b = f(); a*10+b",
	"a, b = [1]; b",
	"a, b = [1, 2, 3]; b",
	"a, b = [1, 2]; a = \"x\"; a",
	"a, b = 1, 2; a*10+b",
	"a, b = 1, \"x\"; b",
	"a, b = [1], 2; b",
	"a = 0; b = 0; a, b = 5, 6; a + b",
	"a = 1; b = 2; a, b = b, a; a*10+b",
	"a = b = 3; a + b",
	"b = 1; a = b = 3; a + b",
	"a = b = c = 2; a + b + c",
	"a = b = [1]; a#1",
	"f(n) := { n += 1; n }; f(3)",
	"f(a, b) := { b = b * 2; a + b }; f(1, 2)",
	"f(xs) := { xs = xs + [1]; count xs }; f([5])",
	"f(n) := { n = n + 1; n }; x = 3; f(x); x",
	"x = for i in [1, 2, 3] { i * 10 }; x",
	"i = 0; w = while i < 3 { i += 1; i * 2 }; w",
	"(for x in [1, 2] { x }) + 1",
	"xs = [1, 2]; for x in xs { x + 1 }",
	"f(n) := for x in [1, 2] { x + n }; f(1)",
	"x = for i in [] { i }; x",
	"i = 5; w = while i < 3 { i += 1 }; w",
	"count [1, 2]",
	"xs = [1, 2, 3]; count(xs)",
	"2 in [1, 2]",
	"3 in [1, 2]",
	"2 in [2, 2]",
	"if 1 in [1, 2] { 7 } else { 8 }",
	"count 2 in [1, 2, 2, 3]",
	"\"b\" in [\"a\", \"b\"]",
	"(2 in [1, 2]) + \"x\"",
	"f(xs, n) := { if n == 0 then count xs else f(xs, n - 1) }; f([1, 2], 2)",
	"count \"abc\"",
	"count \"\"",
	"n = 0; for c in \"héllo\" { n += 1 }; n",
	"s = \"\"; for c in \"ab\" { s = c + s }; s",
	"\"b\" in \"abc\"",
	"count \"a\" in \"banana\"",
	"s = \"x\"; for c in \"ab\" { s = c + 1 }; s",
	"xs = [1, 2]; count xs in [1]",
	"out = []; for x in [[1], [2, 3]] { out = out + x }; out",
	"xs = [1]; xs + xs",
	"xs = [1]; xs + 2",
	"f(x) := x + [1]; f([0])",
	"xs = [1]; ys = [\"a\"]; zs = xs + ys; zs#2",
	"i = 1; j = i++; j*10+i",
	"i = 1; j = ++i; j*10+i",
	"i = 1; i--; i",
	"i = 1; while (i < 10) do { i++ }; i",
	"i = 0; for x in [1, 2] { i++ }; i",
	"f(n) := { k = n; k++; k }; f(4)",
	"p = {x:1 y:2}; p.y",
	"p = {x:1}; p.x + \"a\"",
	"{a:1} == {a:1}",
	"{a:1} == {a:2}",
	"{a:1, b:2} == {b:2, a:1}",
	"{a:1} == {a:1, b:2}",
	"p = {x:1}; q = p; q.x = 5; p.x",
	"p = {x:1}; q = {x:1}; p === q",
	"p = {x:1}; q = p; p same q",
	"p = {x:1}; p.x = \"s\"; p.x",
	"p = {x:1}; p.z = 5; p.z",
	"f(m) := m.x; f({x:3})",
	"p = {x:{y:2}}; p.x.y",
	"d = {year:1970 month:1 day:1}; [d.year, d.month, d.day]",
	"xs = [{a:1} {a:2}]; xs[1].a",
	"class P { x: int }; p = P(1); m = {x: 2}; p.x + m.x",
	"p = {x:1}; p.x - \"a\"",
	"a: int = 0; a, b = 3, \"x\"; a + 1",
	"a: int = 0; a, b = \"xy\", 6; a",

	"f = x => x*2; f(3)",
	"make_adder(n) := (x => x+n); add2 = make_adder(2); add2(5)",
	"t = 7; shout = s => s + t; shout(1)",
	"apply(g, n) := { s=0; for i in 0..n { s = g(s) }; s }; k=1; apply(x => x+k, 3)",
	"n = 0; inc = x => { n += x; n }; inc(2); inc(3)",
	"f = x => x*2; f = y => y + 1; f(3)",
	"x = 1; f = y => x + y; f(1); x = 10; 0",
	"f = x => x*2; f = 3; 0",
	// named arguments go to their parameter, positional ones fill the rest; all run in parameter order
	"f(a, b) := a - b; f(b=1, a=5)",
	"f(a, b) := a - b; f(5, b=1)",
	"f(a: int, b: text) := a; f(b=\"x\", a=5)",
	"f(a: int, b: text) := a; f(b=5, a=\"x\")",
	"global i = 0; g() := { i = i * 10 + 1; i }; h() := { i = i * 10 + 2; i }; f(a, b) := a * 100 + b; f(b=h(), a=g())",
	// a missing argument takes its parameter's default
	"f(a, b=3) := a - b; f(5) + f(5, b=2)",
	"f(a, b: int = 3) := a - b; f(5)",
	"f(a, b: int = 3) := a - b; f(5, b=\"x\")",
	"f(n) := n * 2; f(n=4)",
	// nested functions read the outer function's names; `nonlocal y` writes through, a plain `y = 7` is inner's own
	"outer(x) := { inner(y) := x + y; inner(2) }; outer(5)",
	"outer(x: int) := { inner(y: int) := x + y; inner(\"a\") }; outer(5)",
	"outer(x) := { y = 1; inner() := { nonlocal y; y = y + x }; inner(); inner(); y }; outer(5)",
	"outer(x) := { y = 1; inner() := { y = 7; y }; inner() + y }; outer(5)",
	"outer(x) := { a() := b() + 1; b() := x; a() }; outer(5)",
	"outer() := { y = 1; inc() := { nonlocal y; y += 1 }; twice() := { inc(); inc() }; twice(); y }; outer()",
	"outer(n) := { total = 0; add(k) := { nonlocal total; total += k }; for i in 1..n { add(i) }; total }; outer(4)",
	"outer(n) := { fact(k) := if k < 2 then 1 else k * fact(k - 1); fact(n) }; outer(5)",
	"outer(x) := { mid(y) := { deep(z) := y + z; deep(1) }; mid(10) }; outer(100)",
	// a function may call one defined after it, and two may call each other (warp hoists functions)
	"a() := b() + 1; b() := 5; a()",
	"a() := b() + 1; b() := \"x\"; a()",
	"a(n) := if n < 1 then 0 else b(n - 1); b(n) := a(n); a(3)",
	// `return v` at the end is v; before it, an event whose block handler around the body breaks with v
	"f(x) := { return x + 1 }; f(1)",
	"f(x) := { if x > 2 { return 7 }; 1 }; f(5) + f(1)",
	"f(x) := { for i in 1..x { if i == 3 { return i * 10 } }; 0 }; f(5)",
	"f(n) := { if n < 1 { return 0 }; n + f(n - 1) }; f(4)",
	"f(x: int) := { if x > 2 { return \"big\" }; x }; f(5)",
	// texts compare in codepoint order; min and max of two values
	"\"ab\" < \"b\"",
	"x = \"b\"; x < \"ab\"",
	"min(3, 5) + max(3, 5)",
	"max(\"b\", \"ab\")",
	"xs = [4, 9]; min(count(xs), 1)",
	// `use list`: the module's definitions the program calls come along
	"use list; drop([1, 2, 3], 1)",
	"use list; product([2, 3, 4])",
	"use list; take([1, 2, 3], 2)",
	"use list; product(a, b) := a * b; product(3, 4)",
	// `%` is the Euclidean remainder
	"-7 % 3",
	"7 % -3",
	"5 % 0",
	"n = 7; n %= 3; n",
	"\"a\" % 2",
	// `sum` folds from 0; `.size`, `.count` and `.length` count; a local widens over the numbers it is given
	"sum [1, 2, 3]",
	"xs = [1, 2]; sum xs",
	"sum 1..4",
	"sum = 5; sum + 1",
	"[1, 2, 3].size",
	"xs = [1, 2]; xs.count",
	"\"abc\".length",
	"f() := { out = 1.5; for x in [2, 3] { out = out + x }; out }; f()",
	"f() := { out = 0; out = \"a\"; out }; f()",
	"a: int = 0; a = b = 2.5; a",
	"a: int = 0; a = b = \"x\"; b",
	"class P { x: int }; p = P(1); a = p.x = 5; a + p.x",
	"for x in [1, 2, 3] { if x > 2 then { \"big\" } }",
	"for x in [1, 2, 3] { if x < 2 then { \"small\" } }",
	// `/`: ints that divide stay ints, others make a number; the model types every quotient a number
	"6/2",
	"-9 / 3",
	"7/2",
	"1/0",
	"x = 7; x = x / 2; x",
	"n: number = 7 / 2; n",
	"y: int = 1; y = 7/2; y",
	"\"ab\" / 2",
	// `^`: an int power of ints, a number for a negative exponent
	"2^10",
	"2^0",
	"(-2)^3",
	"2^3^2",
	"x = 3; x^2 + 1",
	"n: number = 2^-1; n",
	"\"ab\"^2",
	// `*` repeats a text a whole number of times, in either order (P1)
	"\"ab\"*3",
	"3*\"ab\"",
	"\"ab\"*0",
	"\"ab\"*-1",
	"\"5\"*3",
	"n = 2; \"ab\"*n",
	"x: text = \"ab\"*2; x",
	"\"ab\"*2.5",
	"\"ab\"*\"cd\"",
	"x: int = \"5\"*3; x",
	// `c ? a : b` is `if c then a else b`
	"x = 3; x > 2 ? \"big\" : \"small\"",
	"x = 1; x < 2 ? 5 : 6.5",
	"x = 1; y: int = x > 2 ? 5 : \"no\"; y",
	"fib(n) := n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)",
	// a number index is checked when it runs
	"xs = [1, 2, 3]; n = 4; xs[n/2]",
	"xs = [1, 2, 3]; n = 3; xs[n/2]",
	// lists are shared (P215): an alias sees each add; an undeclared list takes anything, a declared one its items
	"xs = [1, 2]; ys = xs; ys.add(3); xs",
	"xs = [1]; ys = xs; xs = [5]; ys",
	"xs = []; xs.add(1); xs.add(2); xs.count",
	"xs = [1]; xs.add(\"ab\"); xs",
	"xs: ints = [1]; xs.add(2); xs",
	"xs: ints = [1]; xs.add(\"ab\"); xs",
	// `xs#i = v` writes into the shared list: checked against its element type and its length when it runs
	"xs = [1, 2]; xs#1 = 5; xs",
	"xs = [1]; ys = xs; ys#1 = 7; xs",
	"xs = [1]; xs#3 = 7; xs",
	"xs = [1, 2]; xs#2 = \"ab\"; xs",
	"xs = [1, 2]; xs[0] = 5; xs",
	"xs = [1, 2]; v = (xs#1 = 9); v",
	"xs: ints = [1]; xs#1 = \"ab\"; xs",
	// `e as T` converts: a number keeps its whole part, a text is parsed (or fails when it runs), anything prints as text
	"3.7 as int",
	"\"4\" as int",
	"3 as text",
	"[1] as int",
	"f(x) := x as int; f(\"7\")",
	"f(x) := x as int; f(\"a\")",
	"f(x) := x as text; f([1, 2])",
	// a declared result converts the body's value, as `body as T`, in every spelling
	"def f(x: int) -> int { x + 1 }; f(1)",
	"func f(x: Int) -> Int { return x + 1 }; f(1)",
	"fun f(x: Int): Int { return x + 1 }; f(1)",
	"def f(x: int) -> int: x + 1; f(1)",
	"def half(x) -> int { x / 2 }; half(3)",
	"def half(x) -> float { x / 2 }; half(3)",
	"f(a, b) : int := a - b; f(b=1, a=5)",
	"int square(x) = x*x; square(4) + square 3.1",
	"def square(x) as int = x*x; square 3.1",
	"int half(x) := {y = x; if y > 2 {return y/2}; y/4}; half(3) + half(2)",
	"def f(x) -> int { x }; f(\"a\")",
	"def f(x) -> int { [x] }; f(2)",
	"def f(x) -> text { x }; f(3)",
	"def outer(x) -> int { inner(y) := y * 2; inner(x) / 3 }; outer(5)",
	// a bracketed list type and the fixed widths, a range of ints (card typed-list-elements)
	"xs: [int] = [1, \"a\"]; xs",
	"xs: [int] = [1, 2]; xs",
	"xs: [int16] = [1, 70000]; xs",
	"xs: [int16] = [1, 7]; xs",
	"x: int16 = 70000; x",
	"x: int16 = 5; x",
	"x: int16 = 5; x + 1",
	"x: int16 = \"a\"; x",
	"xs: int16s = [1]; xs.add(70000); xs",
	"xs: int16s = [1]; xs.add(7); xs",
	"x: byte = 5; x = 300; x",
	"x: byte = 5; x = 200; x",
	// quantities: a type per dimension, beside the numbers (card type-theory, units in W0)
	"2 m + 50 cm",
	"2 km - 500 m",
	"1 m < 2 m",
	"50 cm < 1 m",
	"6 m / 2 s",
	"3 * 2 m",
	"6 m / 3 m",
	"1 m == 100 cm",
	"x = 3 m; y = x * 2; y < 7 m",
	"y = 5 m; if y < 6 m { 1 } else { 2 }",
	"if 1 < 2 { 1 m } else { 200 cm }",
	"speed(d, t) := d / t; speed(10 km, 2 h)",
	"1 m + 1",
	"1 m + 1 s",
	"1 m < 2 s",
	"1 m == 1 s",
	"1 m == 1",
	"if 1 < 2 { 1 m } else { 2 s }",
	"if 1 < 2 { 1 m } else { 2 }",
	"\"a\" + 1 m",
	"\"a\" < 1 m",
	"10 m % 3 m",
	"y = 2 m; y < 3",
	"x = 1 m; x - 1",
];

/// Programs warp compiles although the model rejects them: holes in warp's checks, each with its card
const KNOWN_HOLES: &[(&str, &str)] = &[
	("b: bool = no; b++; b", "bool-assign"),
	("f(n) := { n = \"x\"; n }; f(3)", "param-assign-unchecked"),
	("x = 1 m; x = 2 s; x", "units-reassign"),
	("x: int = 1 m; x", "units-annotation"),
	("2 m * \"a\"", "units-text-repeat"),
];

/// Where warp's run-time admission differs from W0's subtyping: a bool is an Int at run time, so an int value passes
/// a bool check (P199 lets only the literals 1 and 0 in; card bool-assign)
const KNOWN_ADMITS_GAPS: [&str; 2] = ["bool ← .int: warp admits true / W0 sub false", "boolean ← .int: warp admits true / W0 sub false"];

/// Programs both accept whose values differ, each with its card
const KNOWN_VALUE_DIFFERENCES: &[(&str, &str)] = &[
	// a covariant alias's write: W0 checks it against the list's own element type when it runs, warp adds 2.5 to ints
	("xs: ints = [1]; ys: numbers = xs; ys.add(2.5); xs", "p215-user"),
	("xs: ints = [1, 2]; ys: numbers = xs; ys#1 = 2.5; xs", "p215-user"),
	// a comparison of quantities gives the int 1, a plain comparison yes
	("1 m < 2 m", "units-compare"),
	("50 cm < 1 m", "units-compare"),
	("1 m == 100 cm", "units-compare"),
	("x = 3 m; y = x * 2; y < 7 m", "units-compare"),
];
/// What the model gives for a program it rejects, and for a value it does not keep
const REJECTED: &str = "rejected";
const UNKEPT: &str = "?";

#[test]
fn test_type_model_is_proved() {
	crate::requires!(crate::common::LEAN);
	for source in model_sources() {
		let text = std::fs::read_to_string(&source).unwrap();
		let unproved = text.lines().find(|line| line.split(|c: char| !c.is_alphanumeric()).any(|word| word == SORRY) || line.trim_start().starts_with(AXIOM_DECLARATION));
		assert!(unproved.is_none(), "{}: {}: the model must be proved", source.display(), unproved.unwrap_or_default());
	}
	let listed = axioms(&THEOREMS).unwrap_or_else(|why| panic!("the model does not build:\n{why}"));
	for line in listed.lines().filter(|line| line.contains("depends on axioms")) {
		let used = line.split('[').nth(1).unwrap_or_default().trim_end_matches(']');
		for axiom in used.split(',').map(str::trim) {
			assert!(STANDARD_AXIOMS.contains(&axiom), "{line}: {axiom} is no axiom of Lean's core logic");
		}
	}
	assert_eq!(listed.lines().filter(|line| line.contains("axioms")).count(), THEOREMS.len(), "{listed}");
}

#[test]
fn test_warp_rejects_what_the_type_model_rejects() {
	crate::requires!(crate::common::LEAN);
	let programs: Vec<&str> = CORPUS.iter().copied().chain(KNOWN_HOLES.iter().map(|(code, _)| *code)).collect();
	let exported: Vec<String> = programs.iter().map(|code| export(code).unwrap_or_else(|why| panic!("{code}: {why}"))).collect();
	let model = verdicts(&exported).unwrap_or_else(|why| panic!("the model does not answer:\n{why}"));
	let mut disagreements = Vec::new();
	for (code, model) in programs.iter().zip(model) {
		let warp = warp_verdict(code);
		let hole = KNOWN_HOLES.iter().find(|(hole, _)| hole == code).map(|(_, card)| card);
		match (&model, &warp, hole) {
			(ModelVerdict::Rejected, Ok(()), None) => disagreements.push(format!("{code}: warp compiles what the model rejects")),
			(ModelVerdict::Accepted(type_name), Err(why), _) => disagreements.push(format!("{code}: warp rejects ({why}), the model accepts it as {type_name}")),
			(ModelVerdict::Rejected, Err(_), Some(card)) => disagreements.push(format!("{code}: hole fixed (card {card}), take it off KNOWN_HOLES")),
			(ModelVerdict::Accepted(_), Ok(()), Some(card)) => disagreements.push(format!("{code}: the model accepts a listed hole (card {card})")),
			_ => {}
		}
	}
	assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

#[test]
fn test_warp_admits_what_the_type_model_subtypes() {
	crate::requires!(crate::common::LEAN);
	let disagreements = admits_disagreements().unwrap_or_else(|why| panic!("the model does not answer:\n{why}"));
	assert_eq!(disagreements, KNOWN_ADMITS_GAPS, "warp's admits and W0's Ty.sub differ");
}

#[test]
fn test_warp_computes_what_the_type_model_computes() {
	crate::requires!(crate::common::LEAN);
	let mut programs: Vec<&str> = CORPUS.to_vec();
	programs.extend(KNOWN_VALUE_DIFFERENCES.iter().map(|(code, _)| *code).filter(|code| !CORPUS.contains(code)));
	let exported: Vec<String> = programs.iter().map(|code| export(code).unwrap_or_else(|why| panic!("{code}: {why}"))).collect();
	let model = outcomes(&exported).unwrap_or_else(|why| panic!("the model does not answer:\n{why}"));
	let mut differences = Vec::new();
	for (code, model) in programs.iter().zip(model) {
		let warp = warp_value(code);
		let compared = model != REJECTED && model != UNKEPT && warp != UNKEPT;
		let known = KNOWN_VALUE_DIFFERENCES.iter().find(|(known, _)| known == code).map(|(_, card)| card);
		match (compared && model != warp, known) {
			(true, None) => differences.push(format!("{code}: warp gives {warp}, the model {model}")),
			(false, Some(card)) => differences.push(format!("{code}: values agree now (card {card}), take it off KNOWN_VALUE_DIFFERENCES")),
			_ => {}
		}
	}
	assert!(differences.is_empty(), "{}", differences.join("\n"));
}
