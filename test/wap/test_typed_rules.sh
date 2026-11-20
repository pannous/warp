#!/usr/bin/env bash
# wap's typed rules (stage 3, decision hole-types): a rule with a hole of a value type fires on code whose type is
# settled, and what it writes is typed again. Run it with bash test/wap/test_typed_rules.sh
source "$(dirname "$0")/compile.sh"

check "a typed rule fires on its type" "$(ran fires 'twice(x: int) := x
rule twice $x: int => 9
twice 3')" "9"
check "a typed rule leaves another type alone" "$(ran other 'twice(x: int) := x
rule twice $x: text => 9
twice 3')" "3"
check "what a typed rule writes is typed again" "$(ran again 'twice(x: int) := x
thrice(x: int) := 8
rule twice $x: int => thrice $x
twice 3')" "8"
check "a typed rule sees the type a parameter without one was called with" "$(ran untyped 'g(x) := x
rule g $x: text => 4
g "a b"')" "4"
# a generic's bare name in a hole takes it with any parts: `$m: map` any map, `$xs: list` any list
check "a hole of a bare generic type takes any of its instances" "$(ran bare_generic 'size_of(x) := 0
rule size_of $m: map => 7
rule size_of $xs: list => 3
print(text(size_of {"a": 1}) + " " + text(size_of [1, 2]) + " " + text(size_of 5))')" "7 3 0"
# a typed rule fires as typing writes the code, so the code around it reads the type the rule gives, and the function
# values typing lifted are never typed again (warp-lead, 2026-11-20, typed-rules-ordering)
check "a typed rule's code is typed where it stands, beside a function value" "$(ran typed_in_place 'g(t: text) as text := t
rule g $x: text => 4
x = g "a b"
add(n: int) := {
	f = y => y + n
	f(5)
}
print(text(x + 1) + " " + text(add(2)))')" "5 7"
exit $((failures > 0))
