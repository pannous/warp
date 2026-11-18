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
exit $((failures > 0))
