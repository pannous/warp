#!/bin/bash
# linear xs = int[n] against the automatic typed array xs = int[n]: n writes then n reads (notes/linear_arrays.md)
warp="${1:-scratch/warp}"
n="${2:-10000000}"
for declared in "linear xs = int[$n]" "xs = int[$n]"; do
	program="$declared; for i in 1 to $n { xs#i = i }; s = 0; for i in 1 to $n { s += xs#i }; s"
	start=$(python3 -c 'import time; print(time.time())')
	result=$("$warp" eval "$program" 2>/dev/null | tail -1)
	echo "$declared: $result in $(python3 -c "import time; print(round(time.time() - $start, 3))") s"
done
