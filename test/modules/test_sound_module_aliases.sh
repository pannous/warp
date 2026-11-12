#!/usr/bin/env bash
# use music and use audio load sound and play like use sound (card sound-module-aliases).
# A got-it note naming sound is not expected: that needs the core module-name alias resolver, which this lib-only
# re-export does not have.
# Run: bash test/modules/test_sound_module_aliases.sh
# The warp is WARP (default the warp on PATH), the lib LIB (default the warp's own, beside its binary; a tree's:
# LIB=<tree>/lib).
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
export WARP_WINDOW=0
export WARP_SOUND=0
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}

check "use music plays" "$("$warp" --lib "$lib" 'use music; play(C4, 0.1s); 1' 2>/dev/null | tail -1)" 1
check "use audio plays" "$("$warp" --lib "$lib" 'use audio; play(C4, 0.1s); 1' 2>/dev/null | tail -1)" 1
check "use sound plays" "$("$warp" --lib "$lib" 'use sound; play(C4, 0.1s); 1' 2>/dev/null | tail -1)" 1
[ $failures = 0 ]
