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
exit $((failures > 0))
