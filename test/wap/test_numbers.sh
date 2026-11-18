#!/usr/bin/env bash
# wap's arithmetic and comparisons (wap/preload/prelude.wap; wiki/operator.md): each operator is a preload function,
# overloaded by the types of its operands. Run it with bash test/wap/test_numbers.sh
source "$(dirname "$0")/compile.sh"

check "float arithmetic and comparison" "$(ran float_sum 'if 1.5 + 1.25 > 2.5 then print "more" else print "less"')" "more"
check "float division" "$(ran float_division 'if 7.0 / 2.0 == 3.5 then print "half" else print "other"')" "half"
check "a call picks the overload of its operands' types" "$(ran overloads 'if 2 * 3 == 6 then print "int" else print "other"
if 2.0 * 3.0 != 6.5 then print "float" else print "other"')" "int
float"
# x op= v is x = x op v for every infix operator of the table (notes/operators.md)
check "compound assignments" "$(ran compound 'x = 2
x += 1
x *= 3
x')" "9"
# an int is any whole number up to the largest of 64 bits (wiki/int.md); a larger one is a float
check "the largest int is an int" "$(ran largest 'print "${9223372036854775807} ${9223372036854775807 - 1}"')" "9223372036854775807 9223372036854775806"
# a float's text is its shortest digits that read back, as the warp runtime prints a float (core/runtime/floats.warp;
# wiki/number.md: 0.1 + 0.2 as a float is 0.30000000000000004)
check "the texts of floats" "$(ran float_texts 'print "${0.1} ${1.5} ${1.0e21} ${-0.0} ${1.0 / 3.0} ${0.1 + 0.2} ${2.5e-5} ${0.000001} ${-2.5}"
print "${5e-324} ${1.0 / 0.0} ${-1.0 / 0.0} ${0.0 / 0.0} ${trunc(2.7)} ${floor(-2.5)} ${ceil(2.1)}"')" "0.1 1.5 1e21 0 0.3333333333333333 0.30000000000000004 0.000025 1e-6 -2.5
5e-324 ∞ -∞ NaN 2 -3 3"
exit $((failures > 0))
