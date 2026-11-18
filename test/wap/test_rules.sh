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
# a hole inside a template text is the spelling of what it took, the longer hole name first (notes/rewrites.md)
check "a hole inside a template text" "$(ran spelled 'rule hail $who $whom => print "hi $who and $whom"
hail ann bob')" "hi ann and bob"
# repetition (notes/rules.md): `($k => $v)...` takes each arm, `print($v)...` writes one print per arm
check "a repeated part matched and written once per match" "$(ran repeat 'rule show { ($k => $v)... } => { print($v)... }
show { 1 => "a"; 2 => "b" }')" "a
b"
check "a repeated key" "$(ran repeat_key 'rule show { ($k: $v)... } => { print($v)... }
show { a: "x"; b: "y" }')" "x
y"
check "a rule writing two statements" "$(ran sequence 'rule both $a $b => print($b); greet() := $a
both "first" "second"
print greet()')" "second
first"
check "a glued pattern rule writing a glued kernel form" "$(ran glued 'rule shout($x) => Block(print($x), print($x))
shout("hi")')" "hi
hi"
exit $((failures > 0))
