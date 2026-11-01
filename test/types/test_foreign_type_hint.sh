#!/usr/bin/env bash
# a foreign type name (the prelude's block of them: Int, String, …) works and hints warp's name, once; a program's own
# type of that name, or an enum case so named, hears nothing. Run it with bash test/types/test_foreign_type_hint.sh;
# the warp is WARP (default the warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
unset WARP_HINTS
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
hints() { "$warp" "$1" 2>&1 | grep '^hint' | sed 's/ at .*//'; }

check "x: Int works" "$("$warp" 'x: Int = 3; x' 2>/dev/null | tail -1)" 3
check "x: Int hints int" "$(hints 'x: Int = 3; x' | grep -c 'Int is int in warp')" 1
check "a class field's String hints text" "$(hints 'class Name(val first: String); Name("Ada").first' | grep -c 'String is text in warp')" 1
check "one hint a name" "$(hints 'f(a: Int, b: Int) := a + b; f(1, 2)' | grep -c 'Int is int')" 1
check "a program's own class Int hears nothing" "$(hints 'class Int { v: text }; x: Int = Int("a"); x.v' | grep -c 'in warp')" 0
check "an enum case Int hears nothing" "$(hints 'enum K { Int, Text }; K.Int' | grep -c 'in warp')" 0
[ $failures = 0 ]
