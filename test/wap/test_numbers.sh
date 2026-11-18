#!/usr/bin/env bash
# wap's arithmetic and comparisons (wap/preload/prelude.wap; wiki/operator.md): each operator is a preload function,
# overloaded by the types of its operands. Run it with bash test/wap/test_numbers.sh
source "$(dirname "$0")/compile.sh"

check "float arithmetic and comparison" "$(ran float_sum 'if 1.5 + 1.25 > 2.5 then print "more" else print "less"')" "more"
check "float division" "$(ran float_division 'if 7.0 / 2.0 == 3.5 then print "half" else print "other"')" "half"
check "a call picks the overload of its operands' types" "$(ran overloads 'if 2 * 3 == 6 then print "int" else print "other"
if 2.0 * 3.0 != 6.5 then print "float" else print "other"')" "int
float"
# parentheses group: the form inside is computed first
check "a group is computed first" "$(ran group '(1 + 2) * 3')" "9"
# x op= v is x = x op v for every infix operator of the table (notes/operators.md)
check "compound assignments" "$(ran compound 'x = 2
x += 1
x *= 3
x')" "9"
exit $((failures > 0))
