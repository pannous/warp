#!/usr/bin/env bash
# A runtime error inside a library's function names its own line in the library's file, never a line of the library
# under the program's name (card wasm-trap: samples/sound/arrangement.warp failed "at arrangement.warp:34:56", a line
# of lib/timers.warp; card positions-always: the place is the library's, lib/timers.warp:…). Run it with bash test/errors/test_library_trap_place.sh; the warp under test is WARP (default
# the warp on PATH), a tree's WARP=<tree>/data/gate/warp
warp=${WARP:-warp}
export WARP_WINDOW=0
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
program=$stage/calls_library.warp
printf '%s\n' 'use timers' 'x = 1' 'timer_at(5, 0, () => 1)' >"$program"
said=$("$warp" "$program" 2>&1 | tail -1)
# (the folder as the system resolves it may differ: /private/var for /var)
case $said in
	*"lib/timers.warp:"*) echo "✓ the trap names the library's own line: $said" ;;
	*) echo "✗ expected the place lib/timers.warp:…, got: $said"; exit 1 ;;
esac
