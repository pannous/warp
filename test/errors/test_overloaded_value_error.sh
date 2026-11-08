#!/usr/bin/env bash
# A function defined more than once, named as a value, means none of its definitions: the refusal names them and
# the lambda that says which (card function-redefinition-error). Run it with bash test/errors/test_overloaded_value_error.sh;
# the warp under test is WARP (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
status=0
says() {
	local code=$1 expected=$2 said
	said=$("$warp" "$code" 2>&1 | tail -1)
	case $said in
		*"$expected"*) echo "✓ $code: $said" ;;
		*) echo "✗ $code: expected …$expected…, got: $said"; status=1 ;;
	esac
}
# passed where the call says how many arguments it gets, it is the lambda calling it, each call taking its variant
says 'quoted(t: text) := "<" + t + ">"; quoted(n: int) := n * 10; [1, 2].map(quoted)' '[10, 20]'
says 'quoted(t: text) := "<" + t + ">"; quoted(n: int) := n * 10; g = quoted; g(2)' 'quoted is defined 2 times'
says 'scaled(x) := x * 2; scaled(x, y) := x * y; g = scaled; g(3)' \
	'scaled is defined 2 times: scaled(x) and scaled(x, y); pass a lambda that calls the one you mean, e.g. x => scaled(x)'
# a module a `use` names is no value, though a function of the same name is overloaded (lib/sound.warp's sound)
works() {
	local code=$1 said
	said=$("$warp" "$code" 2>&1 | tail -1)
	case $said in
		*"is defined"*|*error*|*"outside the core"*) echo "✗ $code: $said"; status=1 ;;
		*) echo "✓ $code: $said" ;;
	esac
}
works 'use draw; use sound; tuning'
# an argument whose parameter the callee calls with one argument is that variant
works 'scaled(x) := x * 2; scaled(x, y) := x * y; apply(f, x) := f(x); apply(scaled, 3)'
# a bare name is the call of the variant taking nothing: the host's random()
works 'random < 1'
# a comprehension's variable and a text pattern's hole are their own names, though functions of the name are overloaded
works 'sum(xs) := 1; sum(a, b) := 2; [max(sum, 1) for sum in [5, 6]]'
works 'post(a, b) := 1; post(a, b, c) := 2; f(p) := match p { "/u/" + id: int + "/p/" + post: int => "\(post)" }; f("/u/3/p/4")'
exit $status
