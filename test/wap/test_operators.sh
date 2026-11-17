#!/usr/bin/env bash
# wap's operator stage (notes/backbone.md "Operators", notes/operators.md): a program printed back by `format` in
# canonical warp, every operator read by the table of wap/preload/operators.wap.
# Run it with bash test/wap/test_operators.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/../native/checks.sh"
compiler="$(cd "$(dirname "$0")/../.." && pwd)/wap/compiler.wap"
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT

formatted() { # formatted <program>: the program as `wap format` prints it (or the compiler's error)
	printf '%s\n' "$1" >"$stage/program.wap"
	$warp "$compiler" format "$stage/program.wap" 2>&1
}
# the program prints as expected
format_check() { check "$1 → $2" "$(formatted "$1")" "$2"; }

format_check '1+2 * 3' '1 + 2 * 3'
format_check '(1 + 2) * 3' '(1 + 2) * 3'
format_check '2^3^2' '2 ^ 3 ^ 2'
format_check '8 - 4 - 2' '8 - 4 - 2'
format_check 'x ≤ y' 'x <= y'
format_check 'a plus b' 'a + b'
format_check 'yes and true' 'yes and yes'
format_check 'x += 1' 'x += 1'
format_check 'f = x => x' 'f = x => x'
format_check 'not a and b' 'not a and b'
# the minus rule: spacing decides prefix or infix, and format spells the reading out
format_check '[a -1]' '[a, -1]'
format_check '[a - 1]' '[a - 1]'
format_check 'a-b' 'a - b'
format_check 'print -x' 'print (-x)'
format_check 'print 1 + 2' 'print 1 + 2'
exit $((failures > 0))
