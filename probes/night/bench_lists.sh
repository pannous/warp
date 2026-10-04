#!/bin/bash
# Timing of list idioms with the freshly built CLI: probes/night/bench_lists.sh [n]
N=${1:-5000}
WARP=/Users/me/.cargo/shared-target.noindex/debug/warp
bench() {
	local start=$(python3 -c 'import time; print(time.time())')
	local result=$("$WARP" eval "$2" 2>/dev/null | tail -1 | cut -c1-60)
	local end=$(python3 -c 'import time; print(time.time())')
	printf '%-28s %6.2fs  %s\n' "$1" "$(python3 -c "print($end-$start)")" "$result"
}
bench "for loop sum" "xs=(0..$N).map(x=>x); s=0; for x in xs { s+=x }; s"
bench "index read loop" "xs=(0..$N).map(x=>x); s=0; for i in 0..$N { s+=xs[i] }; s"
bench "index write loop" "xs=(0..$N).map(x=>x); for i in 0..$N { xs[i]=1 }; sum xs"
bench "map" "xs=(0..$N).map(x=>x*2); count(xs)"
bench "filter" "xs=(0..$N).map(x=>x); count(xs.filter(x=>x>10))"
bench "append loop" "xs=[]; for i in 0..$N { xs.add(i) }; count(xs)"
