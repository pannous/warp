#!/usr/bin/env bash
# all of wap's tests, test/wap/run.sh [--wait] [--stage <n>] [<test scripts>] (default every test/wap/test_*.sh): first wap's
# compiler compiled and run once, or its error, the first signal a worker waits for; then the scripts run detached, niced, as many
# at once as the machine's job slots allow (scripts/job-slot.sh, shared by every worktree), each one's output in data/wap-tests/<script>.txt and one summary line in
# data/wap-tests/summary.txt, read later. --wait runs them in the foreground and prints the summary. The warp under
# test is WARP (default the warp on PATH). The self-build ratchet, ratchet_census.sh (about 4 min), is no default
# script: run it by name, test/wap/run.sh --wait test/wap/ratchet_census.sh, after each wap landing
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
results="$here/../../data/wap-tests"
SUMMARY="$results/summary.txt"
cores=$(getconf _NPROCESSORS_ONLN)
job_slot="$here/../../scripts/job-slot.sh"

# the outputs of the scripts run, each data/wap-tests/<script>.txt
outputs() { for script in "$@"; do echo "$results/$(basename "$script" .sh).txt"; done; }
# the scripts' outputs read into one line: checks passed and failed, the scripts that failed
summed() {
	local passed failed
	passed=$(outputs "$@" | xargs cat | grep -c '^✓')
	failed=$(outputs "$@" | xargs cat | grep -c '^✗')
	echo "wap tests: $passed ✓, $failed ✗ in $# scripts, $(($(date +%s) - started)) s$( [ "$failed" = 0 ] || printf '; failed: %s' "$(outputs "$@" | xargs grep -l '^✗' | xargs -n1 basename | sed 's/\.txt$//' | tr '\n' ' ')")"
}
# every script run in a job slot, niced, given the compiler under test (all_run <module or ""> <scripts>), then the
# summary written; a script waiting for its slot says so in its output
all_run() {
	local module=$1
	shift
	started=$(date +%s)
	printf '%s\n' "$@" | xargs -P "$cores" -I{} bash -c '"$3" bash "$1" ${4:+--compiler "$4"} >"$2/$(basename "$1" .sh).txt" 2>&1' _ {} "$results" "$job_slot" "$module"
	summed "$@" | tee "$SUMMARY"
}

[ "$1" = all_run ] && { shift; all_run "$@"; exit; }
wait=no
built=
while [ $# -gt 0 ]; do
	case $1 in
		--wait) wait=yes; shift ;;
		--stage) built=$2; shift 2 ;;
		*) break ;;
	esac
done
[ $# -gt 0 ] || set -- "$here"/test_*.sh
mkdir -p "$results" && rm -f $(outputs "$@") "$SUMMARY"
source "$here/compile.sh"
# --stage <n>: the compiler under test is wap<n>, wap built by itself n times (milestone 2: the tests pass with wap2)
module=
if [ -n "$built" ]; then
	rm -rf "$results/stage"
	module=$(compiler_built "$results/stage" "$built") || { echo "wap$built: $module"; exit 1; }
	compiler=$module
fi
# the first signal: wap's compiler compiles and runs, reading an empty program
said=$(ran empty '') || { echo "wap compiler: $said"; exit 1; }
echo "wap compiler OK; the checks of $# scripts run$( [ $wait = yes ] || echo " in the background: summary in $SUMMARY")"
if [ $wait = yes ]; then all_run "$module" "$@"; else nohup bash "${BASH_SOURCE[0]}" all_run "$module" "$@" >/dev/null 2>&1 & fi
