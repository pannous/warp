#!/usr/bin/env bash
# wap's calls written with a dot (wiki/function.md: `x.f(a)` is just another way to write `f(x, a)`): `x.f` is
# f(x), `x.f(y)` is f(x, y), any function on any value, a chain read in order. Run it with bash test/wap/test_calls.sh
source "$(dirname "$0")/compile.sh"

check "x.f calls f with x" "$(ran dot 'twice(n: int): int := n
x = 3
x.twice')" "3"
check "x.f(y) calls f with x first" "$(ran dot_arguments 'first(a: int, b: int) := b
3.first(4)')" "4"
check "a dot chain runs in order" "$(ran chain 'one(a: int) := a
two(a: int, b: int) := b
5.one.two(6)')" "6"
check "a dot call on a text" "$(ran on_text 'shout(t: text) := print t
"hey".shout')" "hey"
check "x.f picks the f that takes x's type" "$(ran member_overload 'f(a: int) := 1
f(a: text) := 2
"st".f')" "2"
check "an overload writing more structure is more specific: T? beats T" "$(ran structure_overload 'same(a: T, b: T) as text := "any"
same(a: T?, b: T?) as text := "optional"
x: int? = 3
print(same(x, x))
print(same(1, 2))')" "optional
any"
check "a definition for a named enum wins over one for any enum" "$(ran named_enum_wins 'enum Color { red green }
enum Tone { low high }
==(a: Color, b: Color): bool := yes
print(text(Color.red == Color.green))
print(text(Tone.low == Tone.high))')" "yes
no"
check "a word with several results gives the one that is wanted" "$(ran wanted_result 'hi() as int := 42
hi() as text := "fourtytwo"
t = hi() as int
print text(t)
print(hi() as text)')" "42
fourtytwo"
check "a type's words travel with it: a library's show of a list finds each record's own show" "$(ran own_words 'class P { name: text }
print(show([P("a"), P("b")]))')" '[P{name: "a"}, P{name: "b"}]'
check "a word the program writes wins over the one a rule writes for the same type: a class's own show" "$(ran own_show 'class P { name: text }
show(p: P) as text := "P " + p.name
print(show(P("a")))
print(show([P("b")]))')" "P a
[P b]"
exit $((failures > 0))
