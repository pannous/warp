#!/usr/bin/env bash
# a whole number known when compiling that misfits an int, i64, warns where it enters an int place; a value known only
# at run time, or one that fits, stays silent (extensions/int_misfits.warp, night decision int-wrap-declarations). Run
# it with bash test/native/test_int_misfits.sh; the warp is WARP (default the warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
unset WARP_HINTS
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
warning_of() { "$warp" "$1" 2>&1 | grep '^warning' ; }
fix='; fix: declare nat or number to keep it'

check "a declared int of 10^30 warns" "$(warning_of 'x: int = 10^30; x')" "warning: \`10^30\` does not fit an int (i64): it wraps at 1:10$fix"
check "int x = 2^63 warns" "$(warning_of 'int x = 2^63; x')" "warning: \`2^63\` does not fit an int (i64): it wraps at 1:9$fix"
check "a long numeral warns" "$(warning_of 'x: int = 99999999999999999999; x')" "warning: \`99999999999999999999\` does not fit an int (i64): it wraps at 1:10$fix"
check "an int parameter given 10^30 warns" "$(warning_of 'f(n: int) := n; f(10^30)')" "warning: \`10^30\` does not fit an int (i64): it wraps at 1:19$fix"
check "an int result of 10^30 warns" "$(warning_of 'f(): int := 10^30; f()')" "warning: \`10^30\` does not fit an int (i64): it wraps at 1:13$fix"
check "2^63 - 1 fits" "$(warning_of 'x: int = 2^63 - 1; x')" ''
check "-(2^63) fits" "$(warning_of 'x: int = -(2^63); x')" ''
check "an untyped 10^30 is a nat" "$(warning_of 'x = 10^30; x')" ''
check "an untyped parameter takes 10^30" "$(warning_of 'f(n) := n; f(10^30)')" ''
check "a small int is silent" "$(warning_of 'f(n: int) := n; f(5)')" ''
[ $failures = 0 ]
