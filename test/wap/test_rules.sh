#!/usr/bin/env bash
# wap's shape rules (notes/first_principles.md "Holes"; notes/kernel.md: `unless c { b }` is a rule writing If): a
# spelling written as other code by a rule, from core/sugar or from the program itself, compiled and run.
# Run it with bash test/wap/test_rules.sh
source "$(dirname "$0")/compile.sh"

check "unless runs its body when the condition is no" "$(ran unless 'unless no { print "ran" }')" "ran"
check "unless skips its body when the condition is yes" "$(ran skipped 'unless yes { print "skipped" }
print "end"')" "end"
check "a program's own rule" "$(ran own 'rule twice $body => { $body; $body }
twice { print "hi" }')" "hi
hi"
# a result type after `as` (wiki/function.md: `f() as int := 3` reports its result as int)
check "a result type written with as" "$(ran result 'greet(t: text) as text := t
print greet("hi")')" "hi"
exit $((failures > 0))
