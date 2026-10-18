#!/bin/bash
# Ranks what keeps programs outside emitter.warp's core: every sample and probe runs with WARP_SELF_HOSTED, each
# refusal of the emit stage goes to WARP_EMIT_REFUSALS (src/compiler/self_hosted.rs); a refusal is counted once per
# program. A program inside the core must print what it prints without the switch (the Rust emitter): one that does not
# is listed DIFFERENT. Window samples (raylib, sdl) never run.
# usage: probes/self_hosting/refusals.sh <warp binary> [program path regex]   → data/self_hosting/refusals.txt
set -u
warp=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
filter=${2:-}
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/scratch/refusals
report=$root/data/self_hosting/refusals.txt
PARALLEL_RUNS=4
TIMEOUT_SECONDS=15
rm -rf "$work/runs" && mkdir -p "$work/runs" "$(dirname "$report")"
export WARP=$warp WORK=$work ROOT=$root TIMEOUT_SECONDS
export WARP_SELF_HOSTED=1 WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_CRASH_CARDS=0 WARP_MODULE_CACHE=$work/cache
"$warp" "3 + 4" >/dev/null 2>&1 # compiles the stage programs once, into the cache every run shares
run() { # <program> [setting…]: its output and exit status
	(cd "$(dirname "$1")" && env "${@:2}" timeout "$TIMEOUT_SECONDS" "$WARP" "$1" </dev/null 2>&1; echo "exit $?")
}
refusals_of() { # <program>: its distinct refusals in runs/<n>.txt, "inside" when none
	local name output
	name=$(echo "$1" | tr '/' '_')
	output=$(run "$1" WARP_EMIT_REFUSALS="$WORK/runs/$name.txt")
	[ -s "$WORK/runs/$name.txt" ] && return
	echo inside >"$WORK/runs/$name.txt"
	[ "$output" == "$(run "$1" -u WARP_SELF_HOSTED)" ] || echo "DIFFERENT ${1#$ROOT/}" >"$WORK/runs/$name.different"
}
export -f run refusals_of
find "$root/samples" "$root/probes" -name "*.warp" | grep -E -- "$filter" | grep -vE "raylib|sdl" | sort |
	xargs -P "$PARALLEL_RUNS" -I{} bash -c 'refusals_of "$1"' _ {}
programs=$(ls "$work/runs"/*.txt | wc -l | tr -d ' ')
inside=$(grep -lx inside "$work/runs"/*.txt | wc -l | tr -d ' ')
different=$(cat "$work/runs"/*.different 2>/dev/null)
{
	echo "$programs programs, $inside inside the core (or no emit), $(echo -n "$different" | grep -c .) of them printing otherwise than Rust's"
	[ -n "$different" ] && echo "$different"
	echo "refusals, once per program:"
	for run in "$work/runs"/*.txt; do sort -u "$run"; done | grep -vx inside | sort | uniq -c | sort -rn
} >"$report"
head -40 "$report"
