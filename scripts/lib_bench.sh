#!/bin/bash
# The lib bench: how many of lib/**/*.warp the warp compiler already handles, at two levels:
#   parsed        parser.warp reads the file without an error view (scripts/lib_bench_parse.warp; results only, no
#                 comparison with Rust)
#   pure-compiled the whole file compiles in the pure lane alone (WARP_SELF_HOSTED=pure-only): no refusal, no failed stage
#   Rust          the file compiles in the Rust lane (its module runs); one that doesn't (a runtime part, a file
#                 missing a `use`) is no pure-lane gap and is counted apart
# usage: scripts/lib_bench.sh [warp binary, default main's shared /Users/me/dev/bin/warp] [path regex]
#   → data/lib_bench/<commit>.txt (<commit>-partial.txt with a regex): a line per file "<file>  parse: <verdict>  pure: <verdict>" (each the first error), the
#     summary "parsed X/N, pure-compiled Y/N" and the first errors ranked
set -u
warp=${1:-/Users/me/dev/bin/warp}
filter=${2:-}
root=$(cd "$(dirname "$0")/.." && pwd)
commit=$(git -C "$root" rev-parse --short=10 HEAD)
work=$root/scratch/lib_bench
report=$root/data/lib_bench/$commit${filter:+-partial}.txt
PARALLEL_RUNS=${PARALLEL_RUNS:-4}
FILES_PER_PARSE=${FILES_PER_PARSE:-12}
TIMEOUT_SECONDS=${TIMEOUT_SECONDS:-120}
RAN_PURE="pure: ran" # the line the pure lane writes for a program it compiled (src/compiler/self_hosted.rs)
RAN_RUST="ms  run (" # WARP_PHASE_TIMES' line of a module that ran (src/compiler/phase_times.rs)
rm -rf "$work" && mkdir -p "$work/pure" "$work/rust" "$(dirname "$report")"
export WARP=$warp WORK=$work ROOT=$root TIMEOUT_SECONDS RAN_PURE RAN_RUST
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_CRASH_CARDS=0 WARP_LIB=$root/lib
# a parse batch is one turn of the helper: warp's 60 s compute limit would kill it mid-batch ("crashed")
export WARP_TIME_LIMIT=$((TIMEOUT_SECONDS * FILES_PER_PARSE))

files=$(cd "$root" && find lib -name "*.warp" | grep -E -- "$filter" | sort)
total=$(echo "$files" | grep -c .)

parse_files() { # <file>…: a "<file>\t<verdict>" line each into parse/<first file>.txt
	(cd "$ROOT/scripts" && timeout $((TIMEOUT_SECONDS * $#)) "$WARP" lib_bench_parse.warp "${@/#/../}" 2>/dev/null) |
		grep $'\t' | sed 's|^\.\./||' >"$WORK/parse_$(echo "$1" | tr '/' '_').txt"
}
pure_verdict() { # <file>: "compiled", else the refusal or the first error, into pure/<file>.txt
	local name refusals output status
	name=$(echo "$1" | tr '/' '_')
	refusals=$WORK/pure/$name.refusals
	output=$(cd "$ROOT/$(dirname "$1")" && WARP_SELF_HOSTED=pure-only WARP_EMIT_REFUSALS=$refusals \
		timeout "$TIMEOUT_SECONDS" "$WARP" "$(basename "$1")" </dev/null 2>&1)
	status=$?
	if grep -qvx "$RAN_PURE" "$refusals" 2>/dev/null; then
		echo "refused: $(grep -vx "$RAN_PURE" "$refusals" | head -1)"
	elif grep -qx "$RAN_PURE" "$refusals" 2>/dev/null; then
		echo "compiled"
	elif [ $status == 124 ]; then
		echo "timed out after $TIMEOUT_SECONDS s"
	else
		echo "failed: $(echo "$output" | grep -m1 -E 'Error|error|panick|timeout' || echo "${output:0:200}")"
	fi >"$WORK/pure/$name.txt"
}
rust_verdict() { # <file>: "compiles" when the Rust lane's module ran (its run phase), else the first error, into rust/<file>.txt
	local name output
	name=$(echo "$1" | tr '/' '_')
	output=$(cd "$ROOT/$(dirname "$1")" && WARP_PHASE_TIMES=1 timeout "$TIMEOUT_SECONDS" "$WARP" "$(basename "$1")" </dev/null 2>&1)
	if echo "$output" | grep -q "$RAN_RUST"; then
		echo "compiles"
	else
		# the phase times left out, the error's position too: the same error of several files ranks as one
		output=$(echo "$output" | grep -v "^ *[0-9.]* ms  " | grep -v "^phase times")
		echo "fails: $(echo "$output" | grep -m1 -E 'Error|error|panick|timeout' | sed -E 's/ at [^ "]*[0-9]+:[0-9]+//' || echo "${output:0:200}")"
	fi >"$WORK/rust/$name.txt"
}
export -f parse_files pure_verdict rust_verdict

WARP_SELF_HOSTED=pure-only "$warp" "3 + 4" >/dev/null 2>&1 # the stage programs compile once, warm for every run
echo "$files" | xargs -P "$PARALLEL_RUNS" -n "$FILES_PER_PARSE" bash -c 'parse_files "$@"' _
echo "$files" | xargs -P "$PARALLEL_RUNS" -I{} bash -c 'pure_verdict "$1"' _ {}
echo "$files" | xargs -P "$PARALLEL_RUNS" -I{} bash -c 'rust_verdict "$1"' _ {}

parse_of() { # <file>: its parse verdict, "crashed" when the helper died before it
	local line
	line=$(cat "$work"/parse_*.txt | awk -F'\t' -v file="$1" '$1 == file { print $2; exit }')
	echo "${line:-crashed (the parse helper died before this file)}"
}
{
	parsed=0
	compiled=0
	rust_compiles=0
	for file in $files; do
		parse=$(parse_of "$file")
		pure=$(cat "$work/pure/$(echo "$file" | tr '/' '_').txt")
		rust=$(cat "$work/rust/$(echo "$file" | tr '/' '_').txt")
		[ "$parse" == parsed ] && parsed=$((parsed + 1))
		[ "$pure" == compiled ] && compiled=$((compiled + 1))
		[ "$rust" == compiles ] && rust_compiles=$((rust_compiles + 1))
		printf '%s\tparse: %s\tpure: %s\trust: %s\n' "$file" "$parse" "$pure" "$rust"
	done
	echo
	echo "parsed $parsed/$total, pure-compiled $compiled/$total (of $rust_compiles that Rust compiles) (warp $("$warp" --version 2>/dev/null | head -1), lib at $commit)"
	echo
	echo "first parse errors, ranked:"
	for file in $files; do parse_of "$file"; done | grep -v '^parsed$' | sed -E "s/'.'/'…'/" | sort | uniq -c | sort -rn
	echo
	echo "first pure errors of the files Rust compiles, ranked (the pure lane's gaps):"
	for file in $files; do
		name=$(echo "$file" | tr '/' '_')
		grep -qx compiles "$work/rust/$name.txt" && cat "$work/pure/$name.txt"
	done | grep -v '^compiled$' | sort | uniq -c | sort -rn
	echo
	echo "first Rust errors, ranked (files no lane compiles alone: runtime parts, missing uses):"
	cat "$work"/rust/*.txt | grep -v '^compiles$' | sort | uniq -c | sort -rn
} >"$report"
grep '^parsed ' "$report"
echo "report: $report"
