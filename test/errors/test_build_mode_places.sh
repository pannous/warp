#!/usr/bin/env bash
# The build mode, a word before the program (wiki/cli.md "Build modes", card build-modes): debug, the default, names a
# runtime error's file:line; release, fast and small leave the place out. Run it with bash
# test/errors/test_build_mode_places.sh; the warp under test is WARP (default the warp on PATH)
warp=${WARP:-warp}
here=$(cd "$(dirname "$0")" && pwd)
failed=0
# expect <what> <mode words…>: whether the error names fails.warp:5 (yes or no)
expect() {
	what=$1 named=$2
	shift 2
	said=$(cd "$here/../fixtures/places" && "$warp" --no-window --mute "$@" 'use "fails.warp"; second([1])' 2>&1)
	case $said in *"fails.warp:5:"*) got=yes ;; *) got=no ;; esac
	if [ "$got" = "$named" ]; then echo "✓ $what"; else echo "✗ $what: expected the place named $named, got: $said"; failed=1; fi
}
expect "debug by default names the place" yes
expect "warp debug names the place" yes debug
expect "warp release leaves it out" no release
expect "warp fast leaves it out" no fast
expect "warp small leaves it out" no small
exit $failed
