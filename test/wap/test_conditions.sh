#!/usr/bin/env bash
# wap's If (notes/kernel.md: If; wiki/keyword.md): `if c then a else b`, `if c { a } else { b }`, the else optional. With
# both parts of one type an If has the chosen part's value.
# Run it with bash test/wap/test_conditions.sh
source "$(dirname "$0")/compile.sh"

check "the then part when yes" "$(ran then 'if yes then print "a" else print "b"')" "a"
check "the else part when no" "$(ran else 'if no { print "a" } else { print "b" }')" "b"
check "no else part" "$(ran none 'if no { print "a" }
print "c"')" "c"
check "else if chains" "$(ran chain 'if no then print "a" else if yes then print "b" else print "c"')" "b"
check "an If's value given to a name" "$(ran value 'x = if no then 1 else 2
x')" "2"
check "an If's value as a function's result" "$(ran result 'pick(c: bool) := if c then 10 else 20
pick(yes)')" "10"
check "an if in round brackets is one phrase" "$(ran grouped 'x = (if no then 1 else 2)
x')" "2"
# and, or: the second part runs only when the first does not decide (wiki/operator.md)
check "and skips its second part after no" "$(ran and_skips 'loud(): bool := { print "ran"; yes }
if no and loud() then print "a" else print "b"')" "b"
check "and runs its second part after yes" "$(ran and_runs 'loud(): bool := { print "ran"; yes }
if yes and loud() then print "a" else print "b"')" "ran
a"
check "or skips its second part after yes" "$(ran or_skips 'loud(): bool := { print "ran"; yes }
if yes or loud() then print "a" else print "b"')" "a"
check "not turns a condition round" "$(ran not 'if not no then print "a" else print "b"
if !yes then print "c" else print "d"')" "a
d"
check "an if later in a phrase is the earlier word's argument" "$(ran inner 'print if no then "one" else "two"')" "two"
check "c ? a : b chooses" "$(ran ternary 'x = no ? 1 : 2
x')" "2"
check "a choice as a spaced call's argument" "$(ran ternary_print 'print yes ? "a" : "b"')" "a"
check "a statement guarded after it" "$(ran postfix 'print "shown" if yes
print "hidden" if no
print "end"')" "shown
end"
check "an else on the next line continues the if" "$(ran else_line 'c = no
if c {
	print "a"
}
else if not c { print "d" }
else { print "e" }')" "d"
check "an else indented below continues the if" "$(ran else_indented 'c = no
x = if c then 1
	else 2
print text(x)
if c then print "a"
	else if c then print "b"
	else print "c"')" "2
c"
check "?? runs its right side only when the left is missing" "$(ran default_short 'f(a: int) as int? := if a > 0 then a else ø
g(a: int) as int := {
	x = f(a) ?? return 0
	x * 2 }
print text(g(3))
print text(g(-1))')" "6
0"
check "a block indented below an if is its body" "$(ran if_indented 'c = yes
if c
	print "body"')" "body"
check "an assignment in a then or else part assigns there" "$(ran assign_in_if 'c = no
x = 0
if c then x = 1 else x = 2
print text(x)
xs = [1]
if not c then xs[0] = 5 else xs.add(2)
print text(xs[0])')" "2
5"
check "an if as a list's one item" "$(ran list_if 'xs = [1] + [if no then 2 else 3]
print(text(xs#2))')" "3"
check "a list, a text and an optional are conditions by their truthiness" "$(ran truthiness 'xs = [1]
t = ""
o: int? = ø
if xs { print "list" }
if not t { print "empty text" }
if xs and not o { print "both" }
if o or xs { print "either" }')" "list
empty text
both
either"
check "an optional tested bare is its value where present" "$(ran bare_present 'f(o: int?) := if o then o + 1 else 0
print text(f(2))
print text(f(ø))')" "3
0"
check "0, 0.0, an empty text or list and a missing value are empty, and falsy" "$(ran empty_falsy 'x: int? = ø
print text(0 is empty) + " " + text(0.0 is empty) + " " + text("" is empty) + " " + text([] is empty) + " " + text(x is empty) + " " + text(3 is empty)
print (if 0 then "t" else "f") + (if [1] then "t" else "f") + (if "" then "t" else "f")')" "yes yes yes yes yes no
ftf"
check "an optional holding 0 is falsy" "$(ran optional_zero 'count: int? = 0
print if count then "truthy" else "falsy"')" "falsy"
check "a guard whose block never finishes leaves the variable present for the statements after it, in a loop's body too" "$(ran guard_narrows 'class Escape { reads: int }
found(n: int) as Escape? := if n > 1 then Escape(n) else ø
read(n: int) as int := {
	escape = found(n)
	if escape == ø { return 0 }
	escape.reads + 1
}
counted(n: int) as int := {
	escape = found(n)
	total = 0
	while total < 6 {
		if escape == ø { break }
		total = total + escape.reads
	}
	total
}
print(text(read(5)))
print(text(read(0)))
print(text(counted(2)))
print(text(counted(0)))')" "6
0
6
0"
check "a guard inside a loop's body or a try's part narrows only there: after them the variable may still be missing" "$(ran guard_kept_inside 'class Escape { reads: int }
found(n: int) as Escape? := if n > 1 then Escape(n) else ø
looped(n: int) as text := {
	escape = found(n)
	i = 0
	while i < 0 {
		if escape == ø { break }
		i = i + 1
	}
	if escape == ø then "none" else "some"
}
tried(n: int) as text := {
	escape = found(n)
	r = try { if escape == ø { raise "none" }; escape.reads } else 0
	if escape == ø then "none" else "some"
}
handled(n: int) as text := {
	escape = found(n)
	r = try { 1 } catch e { if escape == ø { return "raised" }; 2 }
	if escape == ø then "none" else "some"
}
print(looped(0) + " " + looped(5))
print(tried(0) + " " + tried(5))
print(handled(0) + " " + handled(5))')" "none some
none some
none some"
check "a variable given a new value after its guard may be missing again" "$(ran guard_reset 'class Escape { reads: int }
found(n: int) as Escape? := if n > 1 then Escape(n) else ø
again(n: int) as text := {
	escape = found(n)
	if escape == ø { return "first" }
	escape = found(n - 4)
	if escape == ø then "none" else "some"
}
print(again(0) + " " + again(5) + " " + again(9))')" "first none some"
check "a guard of either test, a or b, leaves what neither finds for the statements after it" "$(ran guard_or 'class Escape { reads: int }
found(n: int) as Escape? := if n > 1 then Escape(n) else ø
read(n: int) as int := {
	escape = found(n)
	if escape == ø or n > 8 { return 0 }
	escape.reads + 1
}
print(text(read(5)) + " " + text(read(0)) + " " + text(read(9)))')" "6 0 0"
check "a condition of a test and a list, a and xs, is yes when the test holds and the list has items" "$(ran test_and_list 'kept(xs: list<text>, n: int) as text := if n > 0 and xs then "yes" else "no"
print(kept(["a"], 1) + " " + kept([], 1) + " " + kept(["a"], 0))')" "yes no no"
check "no before a value tests it for emptiness, no alone is the literal" "$(ran no_value 'xs: list<int> = []
t = "a"
if no xs { print "no items" }
if no [1] { print "wrong" } else { print "items" }
if no t then print "wrong" else print "a text"
b = no
print(text(b) + " " + text([1, ø].contains(ø)))')" "no items
items
a text
no yes"
check "a condition finds a field of a variable present as it finds the variable" "$(ran field_present 'class Inner { n: int }
class Outer { inner: Inner? }
o = Outer(Inner(3))
if o.inner != ø { print(text(o.inner.n)) }
p = Outer(ø)
if p.inner == ø { print("none") } else { print(text(p.inner.n)) }')" "3
none"
check "what a condition finds of a variable holds in a function value that captures it" "$(ran captured_present 'class Box { n: int }
pick(b: Box?, xs: list<int>) as list<int> := {
	if b == ø { return [] }
	xs.map(x => x + b.n)
}
print(text(pick(Box(10), [1, 2])))')" "[11, 12]"
check "a shape test narrows an optional value to the shape, as it does a present one" "$(ran optional_shape 'f(n: Node?) as text := if n is Text { match n { Text(value) => value } } else "other"
print f(Text("a"))
print f(ø)')" "a
other"
check "a condition finds an item at a written place present, on the right of an or too" "$(ran item_present 'first_or(xs: list<int?>) as int := if xs#1 == ø or xs#1 < 0 then 0 else xs#1 + 1
print(text(first_or([4, ø])) + " " + text(first_or([ø, 2])) + " " + text(first_or([-3])))')" "5 0 0"
# wiki/ambiguity.md "is not": `a is not b` reads `not (a is b)`, of a predicate and of a type alike
check "a is not b is not (a is b)" "$(ran is_not 'ys = [5]
zs: list<int> = []
print(text(ys is not empty) + " " + text(zs is not empty) + " " + text(3 is not text))')" "yes no yes"
exit $((failures > 0))
