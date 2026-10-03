#!/bin/bash
# Cost of unbounded Int: wrapping i64 (compiler before fcbd300b) vs checked fast path vs forced BigInt.
# Usage: probes/bench_unbounded_int.sh [iterations]
# Needs scratch/warp_wrapping (release build of 480ee04b) and scratch/warp_unbounded (current release build).
# Each program runs once with N and once with 1 iteration; the difference is pure execution time.
cd "$(dirname "$0")/../scratch"
N=${1:-10000000}
RUNS=5

NAMES=(sum mulmod fib countdown bignum)
PROGRAMS=(
	'i=0; s=0; while i<N { s+=i; i++ }; s'
	'x=1; i=0; while i<N { x = (x*3) % 1000003; i++ }; x'
	'a=0; b=1; i=0; while i<N { t=a+b; a=b; b=t % 1000000007; i++ }; b'
	'i=N; while i>0 { i-- }; i'
	's=2^70; i=0; while i<N { s+=i; i++ }; s'
)

millis() { python3 -c 'import time; print(int(time.time()*1000))'; }

best_of() { # binary program → best wall time in ms over RUNS
	local best=999999999
	for _ in $(seq $RUNS); do
		local start=$(millis)
		"./$1" "$2" > /dev/null 2>&1
		local elapsed=$(( $(millis) - start ))
		(( elapsed < best )) && best=$elapsed
	done
	echo $best
}

execution_ms() { # binary program n → ms spent executing n iterations
	local program="$2"
	local full=$(best_of "$1" "${program//N/$3}")
	local empty=$(best_of "$1" "${program//N/1}")
	echo $(( full - empty ))
}

printf "%-10s %12s %12s %8s   (N=%d, best of %d)\n" program wrapping unbounded ratio "$N" $RUNS
for index in "${!NAMES[@]}"; do
	name=${NAMES[$index]}
	program=${PROGRAMS[$index]}
	n=$N
	[[ $name == bignum ]] && n=$((N / 1000)) # every BigInt result stays in the append-only heap
	wrapping=$(execution_ms warp_wrapping "$program" $n)
	unbounded=$(execution_ms warp_unbounded "$program" $n)
	ratio=$(python3 -c "print(f'{$unbounded/max($wrapping,1):.2f}')")
	printf "%-10s %10dms %10dms %8s\n" "$name" "$wrapping" "$unbounded" "$ratio"
done
