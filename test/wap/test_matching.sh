#!/usr/bin/env bash
# wap's match (wiki/pattern-matching.md; notes/kernel.md Case): the first arm whose pattern fits the subject gives the
# value; a literal fits the value equal to it, a name fits any value and holds it, `_` fits any.
# Run it with bash test/wap/test_matching.sh
source "$(dirname "$0")/compile.sh"

check "the first literal arm that fits" "$(ran literal 'x = 2
match x { 1 => print "one"; 2 => print "two"; _ => print "many" }')" "two"
check "a match gives its arm's value" "$(ran valued 'size(n: int): text := match n {
	0 => "none"
	1 => "one"
	_ => "many"
}
print size(0)
print size(5)')" "none
many"
check "a name arm holds the subject" "$(ran named 'twice(n: int) := match n { 0 => 0; k => k + k }
twice(21)')" "42"
check "texts are matched by their characters" "$(ran texts 'greet(who: text): text := match who {
	"wap" => "hello, self"
	other => "hello, " + other
}
print greet("wap")
print greet("warp")')" "hello, self
hello, warp"
check "no arm fits: nothing runs" "$(ran none 'match 5 { 1 => print "one" }
print "after"')" "after"
check "a match later in a phrase is the earlier word's argument" "$(ran inner_match 'x = 2
print match x { 1 => "one"; _ => "other" }')" "other"
check "an arm assigns in its result, not to the arrow" "$(ran arm_assigns 'y = 0
x = 1
match x {
	1 => y = 2
	_ => y = 3
}
print text(y)')" "2"
exit $((failures > 0))
