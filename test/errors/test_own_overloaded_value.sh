#!/usr/bin/env bash
# A program's own definitions of a library word, by arity (`twice(x)`, `twice(x, y)` beside lib's twice), are one
# overload set of the program's: each call takes the one of its arity, and the name as a value is refused as any
# overloaded name is. Run it with bash test/errors/test_own_overloaded_value.sh; the warp under test is WARP (default
# the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
status=0
gives() {
	local code=$1 expected=$2 said
	said=$("$warp" "$code" 2>&1 | tail -1)
	case $said in
		*"$expected"*) echo "✓ $code: $said" ;;
		*) echo "✗ $code: expected …$expected…, got: $said"; status=1 ;;
	esac
}
own='twice(x) := x * 5; twice(x, y) := x * y; twice(x, y, z) := x + y + z'
gives "$own; [twice(1), twice(1, 2), twice(1, 2, 3)]" '[5, 2, 6]'
gives 'twice(x, y) := x * y; twice(x) := x * 5; [twice(3), twice(3, 4)]' '[15, 12]'
gives "$own; g = twice; g(3)" \
	'twice is defined 3 times: twice(x), twice(x, y) and twice(x, y, z); pass a lambda that calls the one you mean, e.g. x => twice(x)'
exit $status
