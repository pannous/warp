#!/bin/bash
# Probe for card cache-compiled-shader: every sample run twice through the CLI with a fresh program cache (compile,
# then the kept module) prints the same; a difference means the run reads compile-time state the cache does not keep.
# Usage: probes/program_cache/samples_twice.sh [warp binary] (default scratch/warp). Results: data/program_cache/
set -uo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
warp="${1:-$repo/scratch/warp}"
seconds_per_run=20
results="$repo/data/program_cache"
rm -rf "$results" && mkdir -p "$results/cache"
export WARP_MODULE_CACHE="$results/cache" WARP_NO_WINDOW=1
cd "$repo/samples"

# Paths of temporary files and timings differ from run to run
normalized() { sed -E 's#/(var|tmp|private)/[^ ]*#<path>#g; s/[0-9]+\.[0-9]+ ?(s|ms|µs)\b/<time>/g' "$1"; }

kept=0 differing=0
# samples/<chapter>/<name>.warp; golf/ counts bytes
for sample in */*.warp; do
	[[ $sample == golf/* ]] && continue
	name="$(basename "${sample%.warp}")"
	arguments=()
	[ -f "${sample%.warp}.args" ] && read -ra arguments <"${sample%.warp}.args"
	for run in 1 2; do
		timeout "$seconds_per_run" "$warp" --no-ask "$sample" ${arguments[@]+"${arguments[@]}"} </dev/null >"$results/$name.$run" 2>&1
		echo "exit $?" >>"$results/$name.$run"
	done
	programs_now=$(ls "$results/cache" | grep -c '\.program$')
	[ "$programs_now" -gt "$kept" ] && kept=$programs_now && was_kept=kept || was_kept=compiled
	if ! diff -q <(normalized "$results/$name.1") <(normalized "$results/$name.2") >/dev/null; then
		differing=$((differing + 1))
		echo "DIFFERS ($was_kept): $name"
	fi
done
echo "samples kept: $kept, differing: $differing (outputs in $results)"
