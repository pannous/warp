#!/usr/bin/env bash
# How a value gets to another type (wiki/conversion.md: fitting, repacking, making `T(x)`, testing; wiki/truthiness.md,
# wiki/falsey.md: `empty` is the one word a condition asks; wiki/reference.md: `copy`). One line per decided rule.
# Run it with bash test/wap/test_conversion.sh
source "$(dirname "$0")/compile.sh"

check "a float made into an int is cut, written either way" "$(ran cut 'print text(int(2.7))
print text(2.7 as int)')" "2
2"
check "a bool made from a text is read from it" "$(ran bool_read 'print text(bool("no"))
print text(bool("0"))
print text(bool("yes"))')" "no
no
yes"
check "a bool made from any other value is its test" "$(ran bool_test 'print text(bool([]))
print text(bool(3))')" "no
yes"
check "a condition tests a text, it does not read it" "$(ran text_tested 'print text(if "0" then 1 else 2)')" "1"
check "a copy is a list of its own" "$(ran copied 'a = [1]
b = copy(a)
b.add(2)
print text(#a)
print text(#b)')" "1
2"
check "an instance whose class says nothing counts as yes" "$(ran plain_instance 'class Box { items: list<int> = [] }
print(if Box() then "yes" else "no")')" "yes"
check "a range where a list is wanted" "$(ran range_as_list 'size(xs: list) := #xs
print text(size(1 to 3))')" "3"
check "a text and a number join" "$(ran text_plus_number 'print("a" + 3)
print("x·" + 12)')" "a3
x·12"
exit $((failures > 0))
