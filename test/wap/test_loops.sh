#!/usr/bin/env bash
# wap's loops and returns (notes/kernel.md: While, Return; wiki/keyword.md): `while c { b }` runs b while c holds,
# `return x` leaves the function with x.
# Run it with bash test/wap/test_loops.sh
source "$(dirname "$0")/compile.sh"

check "a loop runs while its condition holds" "$(ran while 'go = yes
while go { print "once"
go = no }
print "done"')" "once
done"
check "an early return's value" "$(ran early 'first(c: bool) := { if c then return 1
2 }
first(yes)')" "1"
check "the end's value when no return runs" "$(ran late 'first(c: bool) := { if c then return 1
2 }
first(no)')" "2"
check "a return in one part of an If" "$(ran part 'f(c: bool) := if c then return 5 else 6
f(yes)')" "5"
check "a counting loop's sum" "$(ran sum 'i = 0
sum = 0
while i < 5 { i = i + 1
sum = sum + i }
sum')" "15"
check "a comparison chooses" "$(ran compare 'x = 3
if x == 3 then print "three" else print "other"')" "three"
check "break leaves the loop" "$(ran break 'i = 0
while yes { i = i + 1
if i == 3 then break }
i')" "3"
check "continue starts the next run" "$(ran continue 'i = 0
sum = 0
while i < 6 { i = i + 1
if i == 2 then continue
sum = sum + i }
sum')" "19"
check "a block of one keyword phrase" "$(ran one 'x = 1
while x < 2 { if x == 1 { x = 2 } }
x')" "2"
# until c { b } is while not c (wiki/keyword.md)
check "until loops until its condition holds" "$(ran until 'x = 0
until x >= 3 { x = x + 1 }
x')" "3"
check "for counts through a range" "$(ran for_range 'sum = 0
for i in 1 to 4 { sum = sum + i }
sum')" "10"
check "for takes each item of a list" "$(ran for_list 'sum(xs: list<int>): int := {
	total = 0
	for x in xs { total = total + x }
	total
}
sum [1, 2, 3, 4]')" "10"
check "continue in a for moves to the next item" "$(ran for_continue 'for x in [1, 2, 3] { if x == 2 then continue
print "item" }')" "item
item"
check "a for inside a for" "$(ran for_nested 'pairs = 0
for i in 1 to 3 { for j in 1 to 2 { pairs = pairs + 1 } }
pairs')" "6"
exit $((failures > 0))
