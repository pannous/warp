#!/usr/bin/env bash
# wap's match (wiki/pattern-matching.md; notes/kernel.md Case): the first arm whose pattern fits the subject gives the
# value; a literal fits the value equal to it, a name fits any value and holds it, `_` fits any.
# Run it with bash test/wap/test_matching.sh
source "$(dirname "$0")/compile.sh"

check "the first literal arm that fits" "$(ran literal 'x = 2
match x { 1 => print "one"; 2 => print "two"; _ => print "many" }')" "two"
check "a match gives its arm's value" "$(ran valued 'size(n: int): text := match n {
	0 => "none"
	1 => "one"
	_ => "many"
}
print size(0)
print size(5)')" "none
many"
check "a name arm holds the subject" "$(ran named 'twice(n: int) := match n { 0 => 0; k => k + k }
twice(21)')" "42"
check "texts are matched by their characters" "$(ran texts 'greet(who: text): text := match who {
	"wap" => "hello, self"
	other => "hello, " + other
}
print greet("wap")
print greet("warp")')" "hello, self
hello, warp"
check "no arm fits: nothing runs" "$(ran none 'match 5 { 1 => print "one" }
print "after"')" "after"
check "a match later in a phrase is the earlier word's argument" "$(ran inner_match 'x = 2
print match x { 1 => "one"; _ => "other" }')" "other"
check "an arm assigns in its result, not to the arrow" "$(ran arm_assigns 'y = 0
x = 1
match x {
	1 => y = 2
	_ => y = 3
}
print text(y)')" "2"
check "a match with a value in one arm and ø in another gives an optional" "$(ran match_optional 'k(n: int, b: bool) := match b {
  yes => n
  _ => ø
}
print text(k(3, yes) == 3)
print text(k(3, no) == ø)')" "yes
yes"
check "a list pattern takes the first item and the rest" "$(ran list_rest 'total(xs: list<int>): int := match xs { [] => 0; [first, ...rest] => first + total(rest) }
print text(total([1, 2, 3]))')" "6"
check "list patterns fit by count" "$(ran list_count 'second(xs: list<text>) := match xs { [a, b] => b; [a, b, ...more] => b + "+"; _ => "none" }
print second(["x", "y"])
print second(["x", "y", "z"])
print second(["x"])')" "y
y+
none"
check "a match on an optional tests its value's cases" "$(ran optional_cases 'enum Shape { Dot, Circle(r: int) }
f(s: Shape?) := match s { Circle(r) => r; Dot => 0; _ => -1 }
print text(f(Circle(3)))
print text(f(Dot))
print text(f(ø))')" "3
0
-1"
check "a declared pattern is a type: a match naming its shape fits every value of it, its holes of their types" "$(ran pattern_type 'pattern Duo = (duo $name: Symbol $items: List)
named(d: Duo) as text := match d { Duo(name, items) => text(name) }
counted(d: Duo) as int := match d { Duo(name, items) => match items { List(parts, bracket, separator) => #parts } }
n: Node = Duo(symbol("a"), List([symbol("x"), symbol("y")], Bracket.Square, Separator.Comma))
print(named(n))
print(text(counted(n)))')" "a
2"
check "a Node where a shape is wanted is told apart at run time" "$(ran shape_at_run_time 'pattern Duo = (duo $name: Symbol $items: List)
kind(d: Duo) as text := "duo"
kind(n: Node) as text := "node"
d: Node = Duo(symbol("a"), List([], Bracket.Square, Separator.Comma))
print(kind(d))
print(kind(symbol("b")))
e: Duo = d
print(text(match e { Duo(name, items) => name }))')" "duo
node
a"
check "a declared pattern's name after is tests the shape, and the then-arm holds a value of it" "$(ran is_shape 'pattern Duo = (duo $name: Symbol $items: List)
named(d: Duo) as text := match d { Duo(name, items) => text(name) }
said(n: Node) as text := if n is Duo then named(n) else "other"
print(said(Duo(symbol("a"), List([], Bracket.Square, Separator.Comma))))
print(said(symbol("b")))')" "a
other"
check "an arm naming a declared pattern on a Node holds its holes of their types: a match inside it on one is whole" "$(ran arm_holes 'pattern Duo = (duo $name: Symbol $items: List)
counted(n: Node) as int := match n {
	Duo(name, items) => match items { List(parts, bracket, separator) => #parts }
	_ => 0
}
print text(counted(Duo(symbol("a"), List([symbol("x"), symbol("y")], Bracket.Square, Separator.Comma))) + counted(symbol("b")))')" "2"
exit $((failures > 0))
