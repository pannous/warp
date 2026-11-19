#!/usr/bin/env bash
# wap's emitter: functions, locals and globals (notes/kernel.md: Define, Set, Call, a name's value). A function's value
# is its body's, a value given to a name is that value again, as warp has it.
# Run it with bash test/wap/test_functions.sh
source "$(dirname "$0")/compile.sh"

check "a value given to a global is the program's value" "$(ran set 'x = 3')" "3"
check "a global read back" "$(ran global 'x = 3
x')" "3"
check "a function gives back its parameter" "$(ran same 'same(n: int) := n
same(5)')" "5"
check "a function's local" "$(ran local 'four() := { y = 4
y }
four()')" "4"
check "a function sees a global set before its call" "$(ran reads 'x = 6
seen() := x
seen()')" "6"
check "round brackets group one value" "$(ran group 'x = (7)
x')" "7"
check "round brackets around statements run them in order" "$(ran statements '(x = 4; x)')" "4"
check "a function value passed and called twice" "$(ran twice 'twice(f, x) := f(f(x))
twice(x => x + 1, 3)')" "5"
check "a function value reads its function's parameter" "$(ran capture 'add(n: int) := {
	g = x => x + n
	g(5)
}
add(2)')" "7"
check "a function value made by a function value keeps what both read" "$(ran nested 'outer(a: int) := {
	b = 10
	make = x => (y => x + y + a + b)
	inner = make(100)
	inner(1000)
}
outer(1)')" "1111"
check "one function value called with an int and a text" "$(ran poly 'id = x => x
print id("ab")
id(3)')" "ab
3"
check "a function value of two parameters" "$(ran pair 'sum = (a, b) => a + b
sum(3, 4)')" "7"
check "the list words take function values" "$(ran words 'limit = 3
print text([1, 2].all(x => x < limit))
print text([1, 5].any(x => x > limit))
print text(["a", "b"].contains("c"))
print text([4, 5].contains(5))')" "yes
yes
no
yes"
check "a call before its function's definition line" "$(ran later 'n = later(2)
later(x: int) := x * 2
n')" "4"
check "a function's name passed as a value" "$(ran named 'inc(x: int) := x + 1
apply(f, x) := f(x)
print text(apply(inc, 2))
g = inc
g(g(5))')" "3
7"
check "the list words take a function's name" "$(ran named_words 'is_small(x) := x < 3
print text([1, 2].all(is_small))
print text([4, 5].any(is_small))')" "yes
no"
check "a call of the value another call gives" "$(ran chained 'adder(n: int) := x => x + n
print text(adder(2)(40))
(x => x * 3)(5)')" "42
15"
check "a parameter of a function type takes a function value, its letters bound by typing the value's function" "$(ran function_types 'apply(f: (int) -> int, x: int) := f(x)
print text(apply(y => y + 1, 4))
twice(f: (T) -> T, x: T): T := f(f(x))
print text(twice(y => y * 3, 2))
print twice(t => t + "!", "ab")')" "5
18
ab!!"
# a definition without parameters is a constant: its name where no function is wanted is its value
check "a constant's name is its value" "$(ran constant 'n := 5
twice(x: int) := x * 2
print text(n + 1)
print text(twice(n))')" "6
10"
check "-> writes a function value as => does (wiki: both arrows denote lambdas)" "$(ran arrow 'g = (a, b) -> a + b
print text(g(1, 2))')" "3"
# code that never finishes, a return, fits any parameter
check "a return given as an argument" "$(ran never_fits 'pick(a: int, b: int) as int := a
g(a: int) as int := pick(a, return 7)
print text(g(1))')" "7"
check "a function calls itself" "$(ran recursive 'f(n: int): int := if n < 1 then 0 else n + f(n - 1)
print text(f(3))')" "6"
# a parameter's default, which a call that leaves it out gives it (a class's field default the same, its constructor's)
check "a parameter's default" "$(ran parameter_default 'f(x: int, y: int = 2) := x + y
print text(f(1) + f(1, 5))
class P { x: int; tag: text = "?" }
print P(3).tag + P(3, "a").tag')" "9
?a"
# a function of a bare list parameter is typed when called, for the list given, so one nothing calls is not refused
check "an uncalled function of a bare list" "$(ran bare_list_uncalled 'first(xs: list) := xs#1
print "ok"')" "ok"
# an argument given by its parameter's name, in any order, the parameters between left to their defaults
check "named arguments" "$(ran named_arguments 'f(a: int, b: int) := a - b
print text(f(b: 1, a: 5))
g(a: int, b: int = 10, c: int = 100) := a + b + c
print text(g(1, c: 2))
class Op3 { spelling: text; kind: text = "" }
print Op3(".", kind: "access").kind
pick(ys: list<text>, by: (text) -> int) := by(ys#1)
print text(["ccc", "a"].pick(by: t => #t))')" "4
13
access
3"
check "a parameter's default names a value of the program" "$(ran default_named 'k = 5
f(g: int = k) := g
print text(f())')" "5"
check "a field's default is a case of an enum" "$(ran field_default_case 'enum Grouping { Left, Right }
class Group { name: text, grouping: Grouping = Grouping.Left }
g = Group("a")
print text(g.grouping == Grouping.Left)')" "yes"
check "a bare function parameter takes any function value" "$(ran bare_function 'twice(f: function, x: int) := f(f(x))
print text(twice(x => x + 1, 3))')" "5"
check "functions of one signature in one list, called as chosen" "$(ran signature_list 'inc(x: int): int := x + 1
double(x: int): int := x * 2
m: list<(int) -> int> = [inc, double, x => x * 3]
for g in m { print text(g(5)) }
print text((m#2)(7))')" "6
10
15
14"
check "function values of a signature keep what they capture" "$(ran signature_captures 'made(n: int): list<(int) -> int> := [x => x + n, x => x * n]
fs = made(10)
print text((fs#1)(1)) + " " + text((fs#2)(2))')" "11 20"
check "a map of functions by name" "$(ran signature_map 'inc(x: int): int := x + 1
twice(x: int): int := x * 2
ops: map<text, (int) -> int> = {"inc": inc, "twice": twice}
print text((ops["twice"])(21))')" "42"
check "an early return of an int in a function giving int?" "$(ran early_optional_return 'first_big(xs: list<int>) as int? := {
	for x in xs { if x > 1 { return x } }
	ø
}
print(text(first_big([1, 5, 2])))
print(text(first_big([1])))')" "5
ø"
check "a result written bare list takes the body's own list type" "$(ran bare_list_result 'items_of(n: Node) as list := match n {
	List(items, bracket, separator) => items
	Empty => []
	_ => [n]
}
listed(n: Node) as list? := match n {
	List(items, bracket, separator) => items
	_ => ø
}
print(text(#items_of(symbol("a"))))
print(text(listed(symbol("a")) == ø))')" "1
yes"
check "a function value given to map over a list is typed by the list's items" "$(ran map_lambda_items 'named(n: text): text := n + "!"
xs: list<text> = ["a", "b"]
print(xs.map(item => named(item)).join(" "))
outer(n: text): text := shown(n, part => named(part))
shown(n: text, f: (text) -> text): text := [n].map(f).join("")
print(outer("c"))')" "a! b!
c!"
check "an early return of ø in a function giving an optional" "$(ran early_return_empty 'class P { a: int }
f(n: int) as P? := {
	m = if n > 0 then n else ø
	k = m ?? return ø
	P(k)
}
print(text((f(3) ?? P(0)).a))
print(text(f(-1) == ø))')" "3
yes"
check "a function taken as a value reads a global set after it when it is called" "$(ran late_global 'H = {"a": f}
X = 1
f(n: int) as int := X + n
g = H["a"]
print(text(g(2)))')" "3"
check "a function reading a global set before the program calls it" "$(ran global_set_first 'f() as int := h() + 1
h() as int := g
g = 3
print(text(f()))')" "4"
check "a λ of a signature called where it is written" "$(ran signature_in_place 'print text(([x => x * 5]#1)(2))')" "10"
check "a λ of a signature taking nothing" "$(ran signature_no_parameters 'fs: list<() -> int> = [() => 5]
print text((fs#1)())')" "5"
check "a λ of a signature keeps what the λ around it captured" "$(ran signature_captures_of_captures 'made(n: int): list<(int) -> list<(int) -> int>> := [x => {
	inner: list<(int) -> int> = [y => x + y + n]
	inner
}]
print text(((made(1)#1)(2)#1)(3))')" "6"
check "code that never finishes adds nothing to a join: an arm that returns, raises or calls a function that always raises" "$(ran jumps 'f(n: int) as int := if n > 0 then n else raise("negative")
g(n: int) := { if n > 2 { return n * 10 }; n }
fail() := raise("always")
y = if f(3) > 9 then fail() else g(5) + g(1)
print text(y)
h(x: int) as text := match x { 1 => "one"; _ => raise("other") }
print(h(1))
print text(f(-1))')" "51
one
negative"
check "a function with no written result joins its returns as an if joins its arms: return ø beside a Node gives a Node?, in one giving nothing it just returns" "$(ran return_join 'found(x: int) := {
	if x > 2 { return ø }
	Text("small")
}
print(if found(1) then "some" else "none")
print(if found(5) then "some" else "none")
class R { error: text = "" }
fail(r: R, why: text) := {
	if r.error != "" { return ø }
	r.error = why
}
r = R()
fail(r, "one")
fail(r, "two")
print(r.error)')" "some
none
one"
check "try gives its body's value, or its else part's where the body raises" "$(ran try_else 'print text(try raise("x") else 7)
f(x: int) as int := if x < 0 then raise("neg") else x
print text(try f(-1) else 0)
print text(try f(5) else 0)')" "7
0
5"
check "a refused function ends only itself: its siblings are typed anyway, each caller is refused naming it, all said together" "$(ran refused_alone 'bad(x: int) as int := x + "a"
worse(x: int) as int := bad(x) + 1
also(x: int) as text := 3
good(x: int) as int := x * 2
print(text(good(21)))' | grep -o 'line [0-9]*:\|calls bad' | tr '\n' ' ')" "line 1: line 2: calls bad line 3: "
check "a program statement wap refuses still lets the functions beside it be typed: all refusals said together" "$(ran refused_beside 'bad(x: int) as int := x + "a"
worse(x: int) as int := bad(x) + 1
also(x: int) as text := 3
print(text(worse(21)))' | grep -o 'line [0-9]*:\|calls [a-z]*' | tr '\n' ' ')" "line 1: line 2: calls bad line 3: line 4: calls worse "
check "try's handler takes the caught Error, its message the raised text, and runs in the function around it: its return returns from that function" "$(ran try_catch 'print(try { raise "boom" } catch e { e.message })
print(text(try { raise "y" } catch { 3 }))
safe(t: text) as int := {
	n = try { if t == "bad" { raise "no" }; 5 } catch e { return 0 }
	n + 1
}
print(text(safe("ok")))
print(text(safe("bad")))')" "boom
3
6
0"
check "an empty catch inside a block drops the error and goes on, as an empty then part does" "$(ran empty_catch 'f(x: int) := if x < 0 then raise("neg") else x
g(c: bool) := { if c { try f(-1) catch e { } }; print "went on" }
g(yes)
if no { }
print "done"')" "went on
done"
check "a function that calls itself and gives no value needs no written result type" "$(ran self_procedure 'countdown(n: int) := {
	if n > 0 {
		print(text(n))
		countdown(n - 1)
	}
}
countdown(3)')" "3
2
1"
check "a function taking a shape is a value where one taking the shape's type is wanted, the shape checked when called" "$(ran shape_value 'pattern Pair = (Pair $left: Node $right: Node)
first_name(items: list<T>, name_of: (T) -> text) as text := name_of(items#1)
pair_name(pair: Pair) as text := "a pair"
items: list<Node> = [Pair(Text("a"), Number(1))]
print first_name(items, pair_name)')" "a pair"
check "a return of a call that never finishes gives nothing back, beside the function's own values" "$(ran return_raising 'refusing(n: int) := raise "no ${n}"
doubled(n: int) as int := {
	if n < 0 { return refusing(n) }
	n * 2
}
print text(doubled(8))')" "16"
check "a function value giving a shape fits where one giving its type is wanted" "$(ran shape_result 'pattern Pair = (Pair $left: Node $right: Node)
applied(n: Node, rewrite: (Node) -> Node) as Node := rewrite(n)
print text(applied(Text("a"), whole => Pair(whole, Number(1))))')" "(Pair \"a\" 1)"
check "a bare function parameter takes a function of a written type" "$(ran bare_function_written 'once(f: function, x: int) := f(x)
inc(x: int): int := x + 1
steps: map<text, (int) -> int> = {"inc": inc}
print text(once(steps["inc"], 3))')" "4"
exit $((failures > 0))
