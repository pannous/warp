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
exit $((failures > 0))
