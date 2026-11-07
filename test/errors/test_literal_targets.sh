#!/usr/bin/env bash
# decision literal-targets: a value where a variable changes, `1 ++ 3`, `1 = 2`, `"a"++`, is refused with a fix naming
# its readings, never guessed. Run it with bash test/errors/test_literal_targets.sh; the warp is WARP (default the
# warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
failures=0

# refused <code> <reading>…: the run fails and its error names every reading
refused() {
	local code=$1 said
	shift
	said=$("$warp" "$code" 2>&1) && { echo "✗ $code ran: $said"; failures=$((failures + 1)); return; }
	for reading in "$@"; do
		if [[ $said != *"$reading"* ]]; then echo "✗ $code: no reading $reading in: $said"; failures=$((failures + 1)); return; fi
	done
	echo "✓ $code"
}

refused '1 ++ 3' 'x = 1, then x++' '1 + +3'
refused '1 -- 3' 'x = 1, then x--' '1 - -3'
refused '1++' 'x = 1, then x++'
refused '1 = 2' 'x = 1, then x = 2' '1 == 2'
refused '1 += 3' 'x = 1, then x += 3'
refused '"a"++' 'x = "a", then x++'
[ "$("$warp" 'x = 1; x++; x' 2>&1)" = 2 ] && echo "✓ x++ counts a variable" || { echo "✗ x++ of a variable"; failures=$((failures + 1)); }
[ $failures = 0 ]
