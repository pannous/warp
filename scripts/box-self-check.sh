#!/usr/bin/env bash
# scripts/box-self-check.sh [--rebuild] [<worktree>]: scripts/self-check.sh's check on the box, with no per-tree cargo
# build (card scripts-off-cargo): the compiler pair of the tree's compiler inputs from data/ or the shared store
# (scripts/rebuild.sh --check; with --rebuild the shared binary builds a missing one, in a heavy slot), then hello world
# through `warp pure` and `warp self` of the shared binary (WARP_BINARY, else /workspace/cow/sol-warp-latest), and
# through warp-host's pure and self when there is one (WARP_HOST_BINARY). A miss without --rebuild is said
# ("REBUILD NEEDED") and the pair there is checked. Exit 0: every lane printed Hello.
# Run it in a heavy slot (/workspace/box-slot.sh scripts/box-self-check.sh <tree>): a --rebuild builds in that slot.
# /workspace/cow/box-self-check.sh and /workspace/selfcheck-box.sh hand a tree that has this script to it.
set -uo pipefail
rebuild=
[ "${1-}" = --rebuild ] && { rebuild=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 1
shared=${WARP_BINARY:-/workspace/cow/sol-warp-latest}
host=${WARP_HOST_BINARY-}
cd "$tree" || exit 1
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$tree/lib
# a copy of the shared binary, taken once, so a refresh of sol-warp-latest never overlaps the check
scratch=$(mktemp -d "${TMPDIR:-/tmp}/box-self-check.XXXXXX") && trap 'rm -rf "$scratch"' EXIT
warp=$scratch/warp
cp -L "$shared" "$warp" || exit 1
if [ -n "$rebuild" ]; then
	pair=$(WARP_SLOT=$scratch WARP_BINARY=$shared "$tree/scripts/rebuild.sh" both "$tree" 2>&1)
else
	pair=$(WARP_BINARY=$shared "$tree/scripts/rebuild.sh" --check both "$tree" 2>&1)
fi
case $? in
	0) ;;
	3) echo "box-self-check: REBUILD NEEDED: no pair for these compiler inputs in data/ nor the store; checking the pair in data/ (--rebuild builds it)" ;;
	*) echo "SELF CHECK RED: scripts/rebuild.sh failed in $tree: $(echo "$pair" | tail -3)"; exit 1 ;;
esac
echo "$pair"
verdict= passed=0
lane() { # lane <name> <command…>: the command prints Hello as its last line
	local name=$1 said
	shift
	said=$("$@" samples/hello/hello.warp 2>&1 | tail -1)
	if [[ $said == *Hello* ]]; then echo "$name: $said"; else verdict+=" $name said: $said;"; passed=1; fi
}
lane "warp pure" "$warp" pure
lane "warp self" "$warp" self
if [ -n "$host" ]; then
	lane "warp-host pure" "$host" --compiler full run
	lane "warp-host self" "$host" run
fi
if [ $passed = 0 ]; then echo "SELF CHECK ok"; else echo "SELF CHECK RED:$verdict"; fi
exit $passed
