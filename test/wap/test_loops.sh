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
# until c { b } is while not c (wiki/keyword.md)
check "until loops until its condition holds" "$(ran until 'x = 0
until x >= 3 { x = x + 1 }
x')" "3"
exit $((failures > 0))
