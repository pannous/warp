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
exit $((failures > 0))
