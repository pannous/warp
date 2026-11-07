#!/bin/bash
# borrow-bootstrap.sh <from lib> <to lib>: a lib/ without runtime.wasm and modules/unicode.wasm starts with <from lib>'s.
# The runtime step's compiler compares texts by NFC, which runs modules/unicode.wasm on runtime.wasm, both written only
# by that step and the next: a fresh lib/ (a new worktree, a CI checkout) builds on an installed or released warp's, as
# warp.wasm builds on the seed, and the steps replace them (card rebuild-fresh-worktree, hGvh0A)
from=${1:?usage: borrow-bootstrap.sh <from lib> <to lib>} to=${2:?usage: borrow-bootstrap.sh <from lib> <to lib>}
[ -f "$to/runtime.wasm" ] && [ -f "$to/modules/unicode.wasm" ] && exit 0
[ -f "$from/runtime.wasm" ] || { echo "borrow-bootstrap.sh: $to has no runtime.wasm and $from none to start from" >&2; exit 0; }
mkdir -p "$to/modules"
for file in "$from"/runtime.wasm "$from"/modules/*.wasm; do
	[ -f "$to/${file#"$from"/}" ] || cp "$file" "$to/${file#"$from"/}"
done
echo "borrow-bootstrap.sh: a fresh $to: its runtime and modules start as $from's, until the build replaces them"
