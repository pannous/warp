#!/bin/bash
# Does Binaryen's wasm-opt make warp's modules faster? Each module optimized at each level, then run alternately with
# the unoptimized one (the Mac is shared: alternate runs see the same load), wasmtime's compile apart from the run.
# usage: probes/perf/wasm_opt_bench.sh <module.wasm> <folder to run in> [rounds, default 2] [-- args of the module]
#   → a line per variant and round: size, wasm-opt time, wasmtime compile ms, run s (the whole warp process)
set -u
WARP=${WARP:-/Users/me/dev/bin/warp}
# wasmtime refuses exact reference types without custom descriptors, which --all-features turns on
FEATURES="--all-features --disable-custom-descriptors"
VARIANTS=("O2:-O2" "O3:-O3" "Oz:-Oz" "O3-inline:-O3 --inline-functions-with-loops --flexible-inline-max-function-size=200 --partial-inlining-ifs=4")
module=$1 folder=$2 rounds=${3:-2}
shift $(($# < 3 ? $# : 3)); [ "${1:-}" = "--" ] && shift
work=$(cd "$(dirname "$0")/../.." && pwd)/scratch/wasm_opt_bench/$(basename "$module" .wasm)
mkdir -p "$work" && cp "$module" "$work/none.wasm"
names=(none)
for variant in "${VARIANTS[@]}"; do
	name=${variant%%:*} flags=${variant#*:}
	start=$(date +%s.%N)
	wasm-opt $FEATURES $flags "$work/none.wasm" -o "$work/$name.wasm" || { echo "$name: wasm-opt failed"; continue; }
	echo "$name: wasm-opt $(echo "$(date +%s.%N) - $start" | bc | cut -c1-5) s"
	names+=("$name")
done
run() { # <variant>: "size compile_ms run_s" of one run, the machine code compiled afresh
	local variant=$1 start
	shift
	local phases=$work/$variant.phases
	start=$(date +%s.%N)
	(cd "$folder" && WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_TIME_LIMIT=900 WARP_MODULE_CACHE=off WARP_PHASE_TIMES=1 \
		"$WARP" "$work/$variant.wasm" "$@" >"$work/$variant.out" 2>"$phases")
	local seconds=$(echo "$(date +%s.%N) - $start" | bc)
	local compile=$(grep "wasmtime compile" "$phases" | awk '{s += $1} END {print s}')
	printf "%-10s %8d bytes  compile %8s ms  whole %6.2f s\n" "$variant" "$(stat -f %z "$work/$variant.wasm")" "$compile" "$seconds"
}
for round in $(seq "$rounds"); do
	for name in "${names[@]}"; do run "$name" "$@"; done
done
for name in "${names[@]}"; do cmp -s "$work/none.out" "$work/$name.out" || echo "$name: output differs (see $work)"; done
