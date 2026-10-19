#!/bin/bash
# Ranks what keeps programs outside emitter.warp's core: every sample and probe runs with WARP_SELF_HOSTED, each
# refusal of the emit stage goes to WARP_EMIT_REFUSALS (src/compiler/self_hosted.rs); a refusal is counted once per
# program. A program inside the core must print what it prints without the switch (the Rust emitter): one that does not
# is listed DIFFERENT. Window samples (raylib, sdl) never run.
# usage: probes/self_hosting/refusals.sh <warp binary> [program path regex]   → data/self_hosting/refusals.txt
# SURVEY=1 FOLDERS=lib/compiler probes/self_hosting/refusals.sh <warp binary>   → data/self_hosting/bootstrap_gap.txt:
#   with WARP_EMIT_SURVEY, every refused function of the programs (warp's compiler itself, card bootstrap-gap) once,
#   ranked by the form refused
set -u
warp=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
filter=${2:-}
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/scratch/refusals
report_name=refusals
[ -n "${SURVEY:-}" ] && report_name=bootstrap_gap
report=$root/data/self_hosting/$report_name.txt
FOLDERS=${FOLDERS:-samples probes}
PARALLEL_RUNS=4
TIMEOUT_SECONDS=15
rm -rf "$work/runs" && mkdir -p "$work/runs" "$(dirname "$report")"
export WARP=$warp WORK=$work ROOT=$root TIMEOUT_SECONDS
[ -n "${SURVEY:-}" ] && export WARP_EMIT_SURVEY=1
export WARP_SELF_HOSTED=1 WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_CRASH_CARDS=0 WARP_MODULE_CACHE=$work/cache
"$warp" "3 + 4" >/dev/null 2>&1 # compiles the stage programs once, into the cache every run shares
run() { # <program> [setting…]: its output and exit status
	(cd "$(dirname "$1")" && env "${@:2}" timeout "$TIMEOUT_SECONDS" "$WARP" "$1" </dev/null 2>&1; echo "exit $?")
}
without_diagnostics() { # the output minus warnings, hints, source excerpts and positions (the self-hosted parser keeps none yet)
	grep -vE '^(warning|hint)|^ *[0-9]* \||^  assumed at ' | sed -E 's/ at [0-9]+:[0-9]+//g'
}
difference_kind() { # <self-hosted output> <Rust output>: parse (parser.warp refused), diagnostics (only they differ) or output
	if echo "$1" | grep -q "^Error(\"Unexpected character"; then echo parse
	elif [ "$(echo "$1" | without_diagnostics)" == "$(echo "$2" | without_diagnostics)" ]; then echo diagnostics
	else echo output; fi
}
refusals_of() { # <program>: its distinct refusals in runs/<n>.txt, "inside" when none
	local name output rust_output
	name=$(echo "$1" | tr '/' '_')
	output=$(run "$1" WARP_EMIT_REFUSALS="$WORK/runs/$name.txt")
	[ -s "$WORK/runs/$name.txt" ] && return
	echo inside >"$WORK/runs/$name.txt"
	rust_output=$(run "$1" -u WARP_SELF_HOSTED)
	[ "$output" == "$rust_output" ] ||
		echo "DIFFERENT $(difference_kind "$output" "$rust_output") ${1#$ROOT/}" >"$WORK/runs/$name.different"
}
export -f run without_diagnostics difference_kind refusals_of
folders=()
for folder in $FOLDERS; do folders+=("$root/$folder"); done
find "${folders[@]}" -name "*.warp" | grep -E -- "$filter" | grep -vE "raylib|sdl" | sort |
	xargs -P "$PARALLEL_RUNS" -I{} bash -c 'refusals_of "$1"' _ {}
programs=$(ls "$work/runs"/*.txt | wc -l | tr -d ' ')
inside=$(grep -lx inside "$work/runs"/*.txt | wc -l | tr -d ' ')
different=$(cat "$work/runs"/*.different 2>/dev/null)
{
	echo "$programs programs, $inside inside the core (or no emit), $(echo -n "$different" | grep -c .) of them printing otherwise than Rust's"
	[ -n "$different" ] && echo "$different" | awk '{print $2}' | sort | uniq -c && echo "$different" | sort
	if [ -n "${SURVEY:-}" ]; then
		refused=$(cat "$work/runs"/*.txt | grep -vx inside | sort -u)
		echo "$(echo -n "$refused" | grep -c .) refused functions, ranked by the form refused:"
		echo "$refused" | sed -E 's/^outside the core: [^ :]+: //' | sort | uniq -c | sort -rn
		echo "each refused function:"
		echo "$refused"
	else
		echo "refusals, once per program:"
		for run in "$work/runs"/*.txt; do sort -u "$run"; done | grep -vx inside | sort | uniq -c | sort -rn
	fi
} >"$report"
head -40 "$report"
