#!/usr/bin/env bash
# a call with an argument count its function does not take is the program's error naming the call the function takes,
# never "outside the core" or an unknown name (card bad-error-message, samples/sound/harmony.warp's `beat 1 1/2`).
# Run it with bash test/errors/test_argument_count.sh; the warp is WARP (default the warp on PATH), its lib/ WARP_LIB
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
failures=0

# refused <code> <shape>: the run fails, names the shape the function takes and no gap of the core
refused() {
	local said
	said=$("$warp" "$1" 2>&1) && { echo "✗ $1 ran: $said"; failures=$((failures + 1)); return; }
	if [[ $said != *"$2"* || $said == *"outside the core"* ]]; then echo "✗ $1: $said"; failures=$((failures + 1)); return; fi
	echo "✓ $1"
}

refused 'f(x) := x
f(1, 2)' 'f(x)'
refused 'f(x) := x
f 1 2' 'f(x)'
refused 'f(x, y = 2) := x + y
f()' 'f(x, y = …)'
refused 'use sound
beat 1 1/2' 'beat(n)'
refused 'use sound
beat(1, 2)' 'beat(n)'
[ "$("$warp" 'f(x) := x * 2
f 21' 2>&1)" = 42 ] && echo "✓ f 21 calls f" || { echo "✗ f 21"; failures=$((failures + 1)); }
[ $failures = 0 ]
