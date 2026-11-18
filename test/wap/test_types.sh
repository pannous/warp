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
print text("ab" === "ab")')" "yes
no
yes"
exit $((failures > 0))
