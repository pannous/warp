#!/usr/bin/env bash
# a call of a word nothing defines is the program's error, `no function from_hex`, its fix the known words sharing a
# part of its name, never "outside the core" (card unknown-from-hex).
# Run it with bash test/errors/test_unknown_function.sh; the warp is WARP (default the warp on PATH), a tree's WARP=<tree>/data/gate/warp
warp=${WARP:-warp}
export WARP_WINDOW=0
failures=0

# refused <code> <said…>: the run fails, says each part and no gap of the core
refused() {
	local code=$1 said part
	shift
	said=$("$warp" "$code" 2>&1) && { echo "✗ $code ran: $said"; failures=$((failures + 1)); return; }
	for part in "$@"; do
		if [[ $said != *"$part"* || $said == *"outside the core"* ]]; then echo "✗ $code: $said"; failures=$((failures + 1)); return; fi
	done
	echo "✓ $code"
}

refused 'from_hex("ff")' 'no function from_hex' 'hex'
refused 'nosuchfn(1)' 'no function nosuchfn'
[ "$("$warp" 'f = x => x + 1
f(2)' 2>&1)" = 3 ] && echo "✓ a lambda's call" || { echo "✗ a lambda's call"; failures=$((failures + 1)); }
[ $failures = 0 ]
