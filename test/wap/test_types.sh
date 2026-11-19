#!/usr/bin/env bash
# wap's typing stage (core/analyze/types.wap): every value's type, and the function each call means. A call picks
# among a name's definitions by its arguments' types, then by the type wanted; a parameter without a type makes one
# version of the function per tuple of argument types. Source: the lead's typing interface (notes/backbone.md).
# Run it with bash test/wap/test_types.sh
source "$(dirname "$0")/compile.sh"

check "a call picks the definition its argument's type fits" "$(ran by_argument 'f(x: int) := 1
f(x: text) := 2
f "ab"')" "2"
check "a call picks the definition whose result is the type wanted" "$(ran by_result 'h(): int := 4
h(): float := 2.5
x: int = h()
x')" "4"
check "a parameter without a type takes each type it is called with" "$(ran untyped 'g(x) := x
g "ab"
g 5')" "5"
check "a written result type" "$(ran result 'same(n: int): int := n
same(6)')" "6"
check "a typed variable" "$(ran typed_variable 'x: int = 3
x')" "3"
check "two typed parameters" "$(ran two 'first(a: int, b: text) := a
first(3, "b")')" "3"
check "a bool beside an int counts as 1 or 0, as == compares loosely" "$(ran loose 'print text(0 == no)
print text(yes == 1)
print text(2 == yes)
print text(no == no)')" "yes
yes
no
yes"
check "a one-letter type is generic, every T of a signature the same, the more specific overload wins" "$(ran letters '===(a: T, b: T) := a == b
===(a, b) := no
first(xs: list<T>): T := xs#1
print text(1 === 1)
print text(0 === false)
print text("ab" === "ab")
print text(first([7, 8]) + 1)
print first(["x", "y"])')" "yes
no
yes
8
x"
check "a one-letter name the program declares is that type, not a generic letter" "$(ran declared_letter 'class P { x: int }
f(p: P) := p.x
print text(f(P(4)) + 1)')" "5"
check "=== is equal and of one type" "$(ran identical 'print text(1 === 1)
print text(0 === no)
print text("ab" === "ab")
print text(1 !== 2)')" "yes
no
yes
yes"
check "an optional int takes an int or ø, its value read where a condition found it present, ?? giving a default" "$(ran optional 'f(x: int?) := if x != ø then x + 1 else 0
print text(f(4) + f(ø))
print text(ø ?? 3)')" "5
3"
# any holds every value, number an int or a float (core/node.warp's Data(value: any))
check "any value fits any" "$(ran any_fits 'f(x: any) := 1
print text(f(3) + f("ab") + f([1]) + f(2.5))')" "4"
check "an int or a float fits a number" "$(ran number_fits 'g(x: number) := 2
print text(g(3) + g(2.5))')" "4"
# a function's written result type is its type, whatever its body gives that fits it
check "the written result type wins" "$(ran written_result 'f(a: int) as int? := ø
h(x: int?) := 1
print text(missing(f(1)))
print text(h(f(1)))')" "yes
1"
check "a value in an any shows itself by its own type's show" "$(ran shown_any 's(x: any): text := show(x)
class Box { n: int }
print s(1) + " " + s(2.5) + " " + s("ab") + " " + s(yes) + " " + s(Box(3))')" "1 2.5 \"ab\" yes Box{n: 3}"
check "a case with an any field shows it" "$(ran held_any 'enum Holder { Held(value: any) }
print show(Held(3))')" "Held(3)"
# a name given only ø holds an optional whose type its uses settle; a generic function given a type not settled yet is
# typed once it is
check "a name given only ø is an optional of the type its uses give" "$(ran missing_name 'held = ø
print text(missing(held))
print text(if missing(held) then 3 else held!)
g(xs: list<T>): int := #xs
xs = []
n = g(xs)
xs += [5]
print text(n + #xs)')" "yes
3
1"
check "a list of any holds values of several types" "$(ran any_list 'xs: list<any> = [1, 2.5, "a b", yes]
for x in xs { print show(x) }')" "1
2.5
\"a b\"
yes"
# `x is w`: a type's word tests the type, a predicate's word calls it (wiki keyword.md: `x = []; x is empty` is yes)
check "is with a predicate word calls it" "$(ran is_predicate 'x = []
print text(x is empty)
print text("a" is empty)
even(n: int): bool := n % 2 == 0
print text(4 is even)
print text(3 is int)')" "yes
no
yes
yes"
# a char literal where a letter bound to text is wanted reads as that text, as where text is written
check "a char literal for a letter bound to text" "$(ran letter_text 'xs: list<text> = ["a", "bc"]
print text(xs.contains("a"))
has_key(m: map<K, V>, key: K): bool := yes
print text({a: 1}.has_key("c"))')" "yes
yes"
check "a type's word after is tests the type, even beside a function of that name" "$(ran is_type_first 'int(t: text): int := 7
print text(3 is int)')" "yes"
check "codepoint is another name of the char type" "$(ran codepoint 'first(c: codepoint): char := c
print text(first('"'"'a'"'"'))')" "a"
check "a number tested for int is an int where the test holds" "$(ran number_narrowed 'f(n: number) := if n is int then n + 1 else 0
print text(f(3))
print text(f(2.5))')" "4
0"
check "a value as a type is the type's function of it" "$(ran as_type 'print (97 as codepoint as text) + (Op.Add as text) + (3 as text)')" "a+3"
check "two lists and two instances are equal by their parts" "$(ran equal_parts 'class P { x: int }
print text([1, 2] == [1, 2])
print text([1, 2] == [1, 3])
print text(P(1) == P(1))
print text(P(1) != P(2))')" "yes
no
yes
yes"
check "an any cast to the type it holds" "$(ran any_cast 'as_int(x: any): int := prim cast(x)
as_text(x: any): text := prim cast(x)
print text(as_int(3) + 1)
print as_text("ab")')" "4
ab"
check "a character is read as its text wherever a text is wanted" "$(ran char_read_as_text 'cs: list<char> = ["a", "b"]
s = ""
s = cs[1]
print(s)
f(at: int) as text := cs[at]
print(f(0))
XS = ["a", "b"]
g(i: int) as text? := {
	if i > 0 { return XS#i }
	ø
}
print(g(1) ?? "none")
print(g(0) ?? "none")')" "b
a
a
none"
check "a program's own class shadows the preload's of the same name" "$(ran own_program_class 'class Program { m: int }
p = Program(3)
print(text(p.m))')" "3"
check "a kind word after is on a Node tests its case, beside a maker of that name; on a text it is the type" "$(ran kind_test 'f(t: Node?) as text := if t is text then "text" else if t is symbol then text(t) else "other"
print(f(Text("b")) + f(symbol("a")) + f(Empty))
g(x: text) as bool := x is text
print(text(g("a")))')" "textaother
yes"
check "a list of optionals is the type of a result, a field or a variable" "$(ran optional_items 'f(x: int) as list<int?> := [x]
class C { ps: list<int?> = [] }
d: list<int?> = [1, 2]
print(text(#f(1)) + " " + text(#C().ps) + " " + text(#d))')" "1 0 2"
exit $((failures > 0))
