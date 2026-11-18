#!/usr/bin/env bash
# all of wap's tests, test/wap/run.sh [--wait] [<test scripts>] (default every test/wap/test_*.sh): first wap's
# compiler compiled and run once, or its error, the first signal a worker waits for; then the scripts run detached, niced, as many
# at once as the machine has cores, each one's output in data/wap-tests/<script>.txt and one summary line in
# data/wap-tests/summary.txt, read later. --wait runs them in the foreground and prints the summary. The warp under
# test is WARP (default the warp on PATH)
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
results="$here/../../data/wap-tests"
SUMMARY="$results/summary.txt"
cores=$(getconf _NPROCESSORS_ONLN)

# the scripts' outputs read into one line: checks passed and failed, the scripts that failed
summed() {
	local passed failed
	passed=$(cat "$results"/test_*.txt | grep -c '^✓')
	failed=$(cat "$results"/test_*.txt | grep -c '^✗')
	echo "wap tests: $passed ✓, $failed ✗ in $# scripts, $(($(date +%s) - started)) s$( [ "$failed" = 0 ] || printf '; failed: %s' "$(grep -l '^✗' "$results"/test_*.txt | xargs -n1 basename | sed 's/\.txt$//' | tr '\n' ' ')")"
}
# every script run, niced, one per core at a time, then the summary written
all_run() {
	started=$(date +%s)
	printf '%s\n' "$@" | nice -n 19 xargs -P "$cores" -I{} bash -c 'bash "$1" >"$2/$(basename "$1" .sh).txt" 2>&1' _ {} "$results"
	summed "$@" | tee "$SUMMARY"
}

[ "$1" = all_run ] && { shift; all_run "$@"; exit; }
wait=no
[ "$1" = --wait ] && { wait=yes; shift; }
[ $# -gt 0 ] || set -- "$here"/test_*.sh
mkdir -p "$results" && rm -f "$results"/test_*.txt "$SUMMARY"
# the first signal: wap's compiler compiles and runs, reading an empty program
source "$here/compile.sh"
said=$(ran empty '') || { echo "wap compiler: $said"; exit 1; }
echo "wap compiler OK; the checks of $# scripts run$( [ $wait = yes ] || echo " in the background: summary in $SUMMARY")"
if [ $wait = yes ]; then all_run "$@"; else nohup bash "${BASH_SOURCE[0]}" all_run "$@" >/dev/null 2>&1 & fi
