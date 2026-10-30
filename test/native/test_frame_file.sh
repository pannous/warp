#!/usr/bin/env bash
# A program FILE with `on frame` runs its frame (card frame-self-rule): a file links the prebuilt standard modules,
# whose frames once called frame_listened from itself (its own rule rewrote its definition), so inline code, compiled
# from source, never showed it. Run it with bash test/native/test_frame_file.sh; the warp under test is WARP (default
# the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
ran() { # ran <name> <code>: the code as a file, run, its output without hints
	printf '%s\n' "$2" >"$stage/$1.warp"
	"$warp" "$stage/$1.warp" 2>&1 | grep -v '^hint:' | tail -1
}

check "on frame in a file" "$(ran frame 'on frame { print(7) }')" "7"
check "a prelude rule inside on frame" "$(ran floats 'x = 3 floats; on frame { print(x) }')" "[0, 0, 0]"
check "a function painted each frame" "$(ran painted 'f(x, y) := [x / 4, y / 4, 0.5]
on frame { paint(f, 4, 4) }' | grep -c '^painted 4×4')" "1"
exit $((failures > 0))
