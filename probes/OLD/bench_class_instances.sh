#!/bin/bash
# Cost of class instances (notes/classes.md "Representation"): N constructions and field reads, N method calls.
# Usage: probes/bench_class_instances.sh [warp binary] [iterations]   default: scratch/warp from scripts/own-warp.sh
# Each program runs with N and with 1 iteration; the difference is the time of the loop alone.
WARP="${1:-$(dirname "$0")/../scratch/warp}"
N=${2:-1000000}
RUNS=3

NAMES=(plain_ints construct_and_read read_only method_call field_write changing_method list_of_instances)
PROGRAMS=(
	'a=3; b=4; i=0; s=0; while i<N { s += a * b; i++ }; s'
	'class Point{x:int; y:int}; i=0; s=0; while i<N { p = Point(i, 2); s += p.x + p.y; i++ }; s'
	'class Point{x:int; y:int}; p = Point(3, 4); i=0; s=0; while i<N { s += p.x * p.y; i++ }; s'
	'class Point{x:int; y:int; sum() := x + y}; p = Point(3, 4); i=0; s=0; while i<N { s += p.sum(); i++ }; s'
	'class Point{x:int; y:int}; p = Point(0, 0); i=0; while i<N { p.x += i; i++ }; p.x'
	'class Counter{n:int; inc() := n += 1}; c = Counter(0); i=0; while i<N { c.inc(); i++ }; c.n'
	'class Point{x:int; y:int}; ps = []; i=0; while i<N { ps.add(Point(i, 1)); i++ }; s=0; for p in ps { s += p.x + p.y }; s'
)

millis() { python3 -c 'import time; print(int(time.time()*1000))'; }

best_of() { # program → best wall time in ms over RUNS
	local best=999999999
	for _ in $(seq $RUNS); do
		local start=$(millis)
		"$WARP" --no-ask eval "$1" > /dev/null 2>&1
		local elapsed=$(( $(millis) - start ))
		(( elapsed < best )) && best=$elapsed
	done
	echo $best
}

for index in "${!NAMES[@]}"; do
	program="${PROGRAMS[$index]}"
	result=$("$WARP" --no-ask eval "${program//N/$N}" 2>&1 | tail -1)
	full=$(best_of "${program//N/$N}")
	once=$(best_of "${program//N/1}")
	printf '%-20s %6d ms for %d   (%s)\n' "${NAMES[$index]}" $(( full - once )) "$N" "$result"
done
