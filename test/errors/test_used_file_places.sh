#!/usr/bin/env bash
# Every error, warning and trap names the file:line of the code it is about, inside a used file or library module too
# (card positions-always). Run it with bash test/errors/test_used_file_places.sh; the warp under test is WARP (default
# the warp on PATH), a tree's WARP=<tree>/data/gate/warp
warp=${WARP:-warp}
here=$(cd "$(dirname "$0")" && pwd)
failed=0
# expect <what> <code> <place>: running the code (from test/fixtures/places) prints the place
expect() {
	said=$(cd "$here/../fixtures/places" && "$warp" --no-window --mute "$2" 2>&1)
	case $said in
		*"$3"*) echo "✓ $1: $3" ;;
		*) echo "✗ $1: expected $3, got: $said"; failed=1 ;;
	esac
}
expect "a refusal in a used file" 'use "../broken_word.warp"; shout("a")' "broken_word.warp:4:"
expect "a warning in a used file" 'use "warns.warp"; half(4)' "warns.warp:3:"
expect "a trap in a used file" 'use "fails.warp"; second([1])' "fails.warp:5:"
expect "a trap in a library module" 'use matrix; Matrix([[1, 2]]) * Matrix([[1, 2]])' "lib/matrix.warp:"
exit $failed
