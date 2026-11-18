#!/usr/bin/env bash
# wap's If (notes/kernel.md: If; wiki/keyword.md): `if c then a else b`, `if c { a } else { b }`, the else optional. With
# both parts of one type an If has the chosen part's value.
# Run it with bash test/wap/test_conditions.sh
source "$(dirname "$0")/compile.sh"

check "the then part when yes" "$(ran then 'if yes then print "a" else print "b"')" "a"
check "the else part when no" "$(ran else 'if no { print "a" } else { print "b" }')" "b"
check "no else part" "$(ran none 'if no { print "a" }
print "c"')" "c"
check "else if chains" "$(ran chain 'if no then print "a" else if yes then print "b" else print "c"')" "b"
check "an If's value given to a name" "$(ran value 'x = if no then 1 else 2
x')" "2"
check "an If's value as a function's result" "$(ran result 'pick(c: bool) := if c then 10 else 20
pick(yes)')" "10"
check "an if in round brackets is one phrase" "$(ran grouped 'x = (if no then 1 else 2)
x')" "2"
# and, or: the second part runs only when the first does not decide (wiki/operator.md)
check "and skips its second part after no" "$(ran and_skips 'loud(): bool := { print "ran"; yes }
if no and loud() then print "a" else print "b"')" "b"
check "and runs its second part after yes" "$(ran and_runs 'loud(): bool := { print "ran"; yes }
if yes and loud() then print "a" else print "b"')" "ran
a"
check "or skips its second part after yes" "$(ran or_skips 'loud(): bool := { print "ran"; yes }
if yes or loud() then print "a" else print "b"')" "a"
check "not turns a condition round" "$(ran not 'if not no then print "a" else print "b"
if !yes then print "c" else print "d"')" "a
d"
check "an if later in a phrase is the earlier word's argument" "$(ran inner 'print if no then "one" else "two"')" "two"
check "c ? a : b chooses" "$(ran ternary 'x = no ? 1 : 2
x')" "2"
check "a choice as a spaced call's argument" "$(ran ternary_print 'print yes ? "a" : "b"')" "a"
check "a statement guarded after it" "$(ran postfix 'print "shown" if yes
print "hidden" if no
print "end"')" "shown
end"
check "an else on the next line continues the if" "$(ran else_line 'c = no
if c {
	print "a"
}
else if not c { print "d" }
else { print "e" }')" "d"
check "an else indented below continues the if" "$(ran else_indented 'c = no
x = if c then 1
	else 2
print text(x)
if c then print "a"
	else if c then print "b"
	else print "c"')" "2
c"
check "?? runs its right side only when the left is missing" "$(ran default_short 'f(a: int) as int? := if a > 0 then a else ø
g(a: int) as int := {
	x = f(a) ?? return 0
	x * 2 }
print text(g(3))
print text(g(-1))')" "6
0"
exit $((failures > 0))
