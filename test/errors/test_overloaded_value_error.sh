#!/usr/bin/env bash
# A function defined more than once, named as a value, means none of its definitions: the refusal names them and
# the lambda that says which (card function-redefinition-error). Run it with bash test/errors/test_overloaded_value_error.sh;
# the warp under test is WARP (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
status=0
refused() {
	local code=$1 expected=$2 said
	said=$("$warp" "$code" 2>&1 | tail -1)
	case $said in
		*"$expected"*) echo "✓ $code: $said" ;;
		*) echo "✗ $code: expected …$expected…, got: $said"; status=1 ;;
	esac
}
refused 'quoted(t: text) := "<" + t + ">"; quoted(n: int) := n * 10; [1, 2].map(quoted)' \
	'quoted is defined 2 times, for (text) and (int); pass a lambda that calls the one you mean, e.g. x => quoted(x)'
refused 'quoted(t: text) := "<" + t + ">"; quoted(n: int) := n * 10; g = quoted; g(2)' 'quoted is defined 2 times'
refused 'scaled(x) := x * 2; scaled(x, y) := x * y; g = scaled; g(3)' \
	'scaled is defined 2 times, with 1 and 2 parameters; pass a lambda that calls the one you mean, e.g. x => scaled(x)'
exit $status
