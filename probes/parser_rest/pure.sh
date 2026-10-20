#!/bin/bash
# runs each probe in the pure-only lane with this worktree's lib: pure.sh <probe.warp>…
here=$(cd "$(dirname "$0")" && pwd)
for probe in "$@"; do
	echo "== $probe"
	WARP_LIB="$here/../../lib" WARP_HINTS=0 WARP_NO_WINDOW=1 WARP_SELF_HOSTED=pure-only timeout 300 /Users/me/dev/bin/warp "$probe" 2>&1 | grep -v "^warning\|^  assumed" | tail -4
done
