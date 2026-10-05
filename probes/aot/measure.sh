#!/bin/bash
# Compile vs run time of warp programs on the wasmtime CLI: JIT (.wasm) against precompiled (.cwasm)
# usage: probes/aot/measure.sh <dir with .wasp files> [warp binary]
set -e
DIR=${1:-scratch/aot}
WARP=${2:-scratch/warp}
FLAGS=(-W gc=y,function-references=y -C cache=n)
RUNS=10
best_ms() { # best wall time of RUNS runs of the command, in ms
	local best=999999
	for _ in $(seq $RUNS); do
		local start=$(perl -MTime::HiRes=time -e 'printf "%.0f", time*1e6')
		"$@" >/dev/null 2>&1 || true
		local end=$(perl -MTime::HiRes=time -e 'printf "%.0f", time*1e6')
		local took=$(( (end - start) / 1000 ))
		(( took < best )) && best=$took
	done
	echo $best
}
printf "%-18s %8s %8s %9s %8s %8s\n" program wasm_B cwasm_B compile_ms jit_ms aot_ms
for source in "$DIR"/*.wasp; do
	name=${source%.wasp}
	"$WARP" compile "$source" >/dev/null
	wasmtime compile "${FLAGS[@]}" "$name.wasm" -o "$name.cwasm"
	compile=$(best_ms wasmtime compile "${FLAGS[@]}" "$name.wasm" -o "$name.cwasm")
	jit=$(best_ms wasmtime run "${FLAGS[@]}" --invoke main "$name.wasm")
	aot=$(best_ms wasmtime run "${FLAGS[@]}" --allow-precompiled --invoke main "$name.cwasm")
	printf "%-18s %8d %8d %9d %8d %8d\n" "$(basename $name)" $(stat -f %z "$name.wasm") $(stat -f %z "$name.cwasm") $compile $jit $aot
done
