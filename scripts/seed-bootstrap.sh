#!/bin/bash
# seed-bootstrap.sh <warp> <seed> <lib>: a lib/ without runtime.wasm and modules/unicode.wasm gets the seed's.
# The runtime step's compiler compares texts by NFC, which links modules/unicode.wasm behind runtime.wasm, both written
# only by that step and the next: a fresh lib/ (a new worktree, a CI checkout) starts with the pair the seed builds of
# this lib/, as warp.wasm is built behind the seed's runtime, and the steps replace them (cards rebuild-fresh-worktree,
# hGvh0A: a released warp's pair is too old, v1.7.0 has no unicode.wasm and its compiler can't build this lib/'s)
usage="usage: seed-bootstrap.sh <warp> <seed> <lib>"
warp=${1:?$usage} seed=${2:?$usage} lib=${3:?$usage}
[ -f "$lib/runtime.wasm" ] && [ -f "$lib/modules/unicode.wasm" ] && exit 0
mkdir -p "$lib/modules"
WARP_LIB=$lib "$warp" "$seed" runtime "$lib/runtime.wasm" &&
	WARP_LIB=$lib "$warp" "$seed" module unicode "$lib/runtime.wasm" "$lib/modules/unicode.wasm" ||
	{ echo "seed-bootstrap.sh: $seed built no runtime.wasm and modules/unicode.wasm for $lib" >&2; exit 1; }
echo "seed-bootstrap.sh: a fresh $lib: its runtime and unicode module are the seed's, until the build replaces them"
