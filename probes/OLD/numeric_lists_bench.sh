#!/bin/bash
# Where big lists of numbers are still boxed Node cells (card compiler-picks): the same work in the shapes programs
# write it, each timed as a CLI eval with the module cache off (compile included, a few 0.01 s).
# Usage: probes/numeric_lists_bench.sh [n] [warp binary]
set -euo pipefail
N="${1:-1000000}"
WARP="${2:-$(dirname "$0")/../scratch/warp}"
export WARP_MODULE_CACHE=off
FILL="xs = []; for i in 1..n { xs.add(i) }"

bench() {
	local start end result
	start=$(perl -MTime::HiRes=time -e 'print time')
	result=$("$WARP" eval "n = $N; $2" 2>&1 | tail -1 | cut -c1-30)
	end=$(perl -MTime::HiRes=time -e 'print time')
	printf "%-12s %-32s %.2f s\n" "$1" "$result" "$(echo "$end - $start" | bc)"
}

bench local "$FILL; sum xs"
bench returned "make(n) := { $FILL; xs }; ys = make(n); sum ys"
bench parameter "total(xs) := { s = 0; for x in xs { s += x }; s }; $FILL; total(xs)"
bench floats "xs = []; for i in 1..n { xs.add(i * 0.5) }; sum xs"
bench map "xs = (1..n).map(x => x * 2); sum xs"
bench global "$FILL; at(i) := xs#i; s = 0; for i in 1..n { s += at(i) }; s"
bench filled "xs = int[n]; for i in 1..n { xs#i = i }; sum xs"
bench sorted "xs = []; for i in 1..n { xs.add(n - i) }; ys = sort xs; ys#1"
