#!/usr/bin/env bash
# scripts/samples.sh [<tree>, default this checkout]: how many of the tree's samples/ run, the gap to 2.0 (decision
# release-1-5-tonight). Each samples/**/*.warp runs from the tree's root through the installed warp (WARP_NO_WINDOW,
# CPU_SECONDS of CPU each); one fails when the native warp exits other than 0, its error the first line it wrote to
# stderr, and one that hits a limit is TIME, not FAIL. Writes test/samples.txt, tracked (status, sample and a failure's
# reason, its positions and quoted texts left out, one a line: after-push commits it when it changed, so git diff and
# git log -p of it name the samples that changed), data/samples/report.txt (each failure's whole first error line) and
# each run's output to data/samples/out/ (<sample>.txt, <sample>.err); prints the count and the failures grouped by
# their reason.
set -uo pipefail
source "$(dirname "$0")/lib/platform.sh"
source "$(dirname "$0")/lib/results-header.sh"
WARP=${WARP:-$warp_binary}
# a sample's budget is CPU time, which a loaded machine doesn't shrink (a 30 s wall clock timed out the sound samples
# beside three builds, 11-03 and 11-04: harmony takes 11 s alone); the wall clock only ends a sample that waits forever
CPU_SECONDS=30
WALL_SECONDS=60
CPU_LIMIT_HIT=152  # 128 + SIGXCPU
PARALLEL=${WARP_SAMPLES_PARALLEL:-1}  # user 2026-11-01: four warps at once, the sweep is the one test run beside 3 builds
# a sample calling a C window library opens its window whatever WARP_NO_WINDOW says: not run, counted apart (until
# FFI windows honour WARP_NO_WINDOW, warp-native-harness)
WINDOW_LIBRARIES='raylib|SDL|glfw'
# a server sample serves until its timeout once it printed this (lib/web.warp serve): served, counted apart; each on a
# port the system chooses (SERVER_PORT 0), so samples run at once, other runs and tests never collide
SERVING='^serving http://'
SERVER_PORT=0
# a sample that fails on purpose says why on a line of its own: EXP instead of FAIL (the page sweep reads it too,
# web/playground/test_in_browser.py EXPECTED_FAILURE)
EXPECTED_FAILURE='^# expected failure: '

tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
results=$tree/data/samples
mkdir -p "$results/out"
# the tree's own compilers, else a count names the tree's commit but measures whichever compiler is installed (128 to
# 122 at 08:12 was the installed one's, the trees' own went 131 to 132)
[ -f "$tree/lib/warp.wasm" ] && export WARP_LIB=${WARP_LIB:-$tree/lib}

# one <sample>: its line of the results
one() {
	local out=$results/out/$(echo "$1" | tr / _)
	grep -qE "$WINDOW_LIBRARIES" "$tree/$1" && { echo "WIN  $1"; return; }
	(cd "$tree" && ulimit -t "$CPU_SECONDS" && WARP_NO_WINDOW=1 WARP_PORT=$SERVER_PORT timeout "$WALL_SECONDS" "$WARP" "$1" </dev/null >"$out.txt" 2>"$out.err")
	local status=$? first verdict=FAIL
	[ $status -eq 124 ] && grep -aq "$SERVING" "$out.txt" && { echo "SRV  $1"; return; }
	[ $status -eq 0 ] && { echo "ok   $1"; return; }
	first=$(grep -av '^wasm-rt warning\|^warning: ' "$out.err" | grep -am1 . | tr '\t' ' ' | cut -c1-200)
	[ $status -eq 124 ] && { verdict=TIME; first="still running after ${WALL_SECONDS} s"; }
	[ $status -eq $CPU_LIMIT_HIT ] && { verdict=TIME; first="used its ${CPU_SECONDS} s of CPU"; }
	grep -q "$EXPECTED_FAILURE" "$tree/$1" && verdict=EXP
	printf '%-4s %s: %s\n' "$verdict" "$1" "${first:-exit $status, nothing on stderr}"
}
export -f one
export tree results WARP CPU_SECONDS WALL_SECONDS CPU_LIMIT_HIT WINDOW_LIBRARIES SERVING SERVER_PORT EXPECTED_FAILURE

report=$results/report.txt
(cd "$tree" && find samples -name '*.warp' | sort) | xargs -P"$PARALLEL" -I{} bash -c 'one "$@"' _ {} | sort -k2 >"$report"
# a reason without its position and quoted texts: one cause reads alike in every sample and from run to run
sed 's/"[^"]*"/"…"/g; s/ ([0-9]*)//g; s/ at samples\/[^ ]*//; s/ (no owner)//' "$report" >"$tree/test/samples.txt"
results_header "$tree/test/samples.txt" "$(git -C "$tree" rev-parse --short=10 HEAD)" 'ok|SRV'
echo "samples: $(grep -c '^ok\|^SRV' "$report") of $(wc -l <"$report" | tr -d ' ') run, $(grep -c '^WIN' "$report") need a window, $(grep -c '^SRV' "$report") serve, $(grep -c '^EXP' "$report") fail on purpose, $(grep -c '^TIME' "$report") hit a limit ($WARP, $(git -C "$tree" rev-parse --short=10 HEAD)) -> $tree/test/samples.txt"
grep -aE '^(FAIL|TIME)' "$tree/test/samples.txt" | sed 's/^[A-Z]* *[^:]*: //' | sort | uniq -c | sort -rn
