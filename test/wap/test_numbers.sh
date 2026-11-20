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
# an int is any whole number up to the largest of 64 bits (wiki/int.md); a larger one is a float
check "the largest int is an int" "$(ran largest 'print "${9223372036854775807} ${9223372036854775807 - 1}"')" "9223372036854775807 9223372036854775806"
# a float's text is its shortest digits that read back, as the warp runtime prints a float (core/runtime/floats.warp;
# wiki/number.md: 0.1 + 0.2 as a float is 0.30000000000000004)
check "the texts of floats" "$(ran float_texts 'print "${0.1} ${1.5} ${1.0e21} ${-0.0} ${1.0 / 3.0} ${0.1 + 0.2} ${2.5e-5} ${0.000001} ${-2.5}"
print "${1.0 / 0.0} ${-1.0 / 0.0} ${0.0 / 0.0} ${trunc(2.7)} ${floor(-2.5)} ${ceil(2.1)}"')" "0.1 1.5 1e21 0 0.3333333333333333 0.30000000000000004 0.000025 1e-6 -2.5
∞ -∞ NaN 2 -3 3"
check "the smaller and the larger of two ints" "$(ran min_max 'print text(min(3, 2)) + " " + text(max(3, 2))')" "2 3"
check "% of ints is Euclidean, the remainder never negative, and ÷ its quotient (wiki/Footguns.md)" "$(ran euclidean 'print text(-7 ÷ 2)
print text(-7 % 3)
print text(7 % -3)
print text(7 ÷ -3)')" "-4
2
1
-2"
check "a power of an int and of a float" "$(ran power 'print text(2 ^ 10)
print text(1.5 ^ 2)
print text(2.0 ^ -1)')" "1024
2.25
0.5"
check "an int's sign turned by a leading -" "$(ran int_negated 'x = 5
print text(-x + 1)')" "-4"
check "a float read from its text" "$(ran float_read 'print text(float("2.5") + float("-0.25") + float("1e3") + float("12"))')" "1014.25"
check "a number read from a text beyond an int's range" "$(ran number_read 'f(x: number) := if x is float then "float" else "int"
print f(number("99999999999999999999"))')" "float"
check "a character made from its code" "$(ran char_made 'print text(char(97))')" "a"
check "a character's code, and a float's 64 bits as an int" "$(ran codes "print text(ord('a'))
print text(float_bits(1.0))")" "97
4607182418800017408"
check "a value made into its own type is itself" "$(ran identity 'print text(int(3)) + " " + text(float(2.5)) + " " + text(bool(no)) + " " + text(4 as int) + " " + text("a b" as text)')" "3 2.5 no 4 a b"
check "a value as a bool: no and 0 written as text are no, any other value its truth" "$(ran bool_of 'print text(bool("no")) + " " + text(bool("0")) + " " + text(bool("yes")) + " " + text(bool("")) + " " + text(bool([])) + " " + text(bool(3)) + " " + text("no" as bool)')" "no no yes no no yes no"
check "a float's whole part, toward zero" "$(ran int_of_float 'print text(int(2.7)) + " " + text(2.7 as int) + " " + text(int(-2.7))')" "2 2 -2"
check "rem and ⦼ give the truncated remainder, of the dividend's sign (wiki/Footguns.md)" "$(ran truncated 'print text(-7 rem 3) + " " + text(-7 ⦼ 3) + " " + text(7 rem -3)')" "-1 -1 1"
# an int widens to a float where one is wanted (wiki/type.md "an int widening to a float is always fine")
check "an int widens to a float: compared, added, passed, and a number against an int" "$(ran widening 'below(v: number) as bool := v < 0
half(x: float) as float := x / 2.0
print text(2.5 < 0) + " " + text(2.5 + 1) + " " + text(half(3)) + " " + text(below(-2.5)) + " " + text(below(2))')" "no 3.5 1.5 yes no"
exit $((failures > 0))
