#!/bin/bash
# A range passed to a function that only reads it (notes/lazy_ranges.md): with bounds passing the call collects no
# list, so the time no longer grows with the range. Usage: probes/range_argument_bench.sh [warp binary]
set -euo pipefail
WARP="${1:-$(dirname "$0")/../scratch/warp}"
for n in 100000 1000000 10000000; do
	code="def total(xs) = sum xs; n = $n; total(1..n)"
	start=$(perl -MTime::HiRes=time -e 'print time')
	result=$("$WARP" eval "$code" 2>&1 | tail -1)
	end=$(perl -MTime::HiRes=time -e 'print time')
	printf "n=%-9s %-22s %.2f s\n" "$n" "$result" "$(echo "$end - $start" | bc)"
done
