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
exit $((failures > 0))
