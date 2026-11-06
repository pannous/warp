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
# beside three builds, 11-03 and 11-04: harmony takes 11 s alone); the wall clock only ends a sample that waits forever,
# WALL_SECONDS stretched by the runnable processes per core (load 40 on 10 cores gave a sample a quarter of one, so its
# 30 s of CPU outlasted 120 s: the nightly of 11-05)
CPU_SECONDS=30
WALL_SECONDS=120  # core/native/tests.warp keeps the same rule for `warp test`
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
# the registry's packages the package samples use by path (modules/packages, text/uniscript_text), as full-suite.sh links them
"$tree/scripts/link-packages.sh" "$tree" >/dev/null || echo "samples: no packages linked: the package samples fail" >&2
# the compiled module wasm/import_compiled links (samples/wasm/shapes.wasm, a link to it), built behind this runtime.wasm
(cd "$tree" && "$WARP" build --module test/fixtures/compiled/shapes.warp >/dev/null) || echo "samples: no test/fixtures/compiled/shapes.wasm: wasm/import_compiled fails" >&2

wall_seconds() {
	local load cores
	load=$(env LC_ALL=C uptime | sed -E 's/.*load averages?: *([0-9.]+).*/\1/')
	cores=$(getconf _NPROCESSORS_ONLN)
	awk -v wall="$WALL_SECONDS" -v load="$load" -v cores="$cores" 'BEGIN { share = load / cores; printf "%d", wall * (share > 1 ? share : 1) }'
}
# one <sample>: its line of the results
one() {
	local out=$results/out/$(echo "$1" | tr / _) wall
	grep -qE "$WINDOW_LIBRARIES" "$tree/$1" && { echo "WIN  $1"; return; }
	wall=$(wall_seconds)
	# what a sample keeps (lib/storage.warp, <sample>.database.sqlite beside it) starts anew each run, as the page's
	# storage does: a counter prints its first run's value, not the sweep count's
	rm -f "$tree/${1%.warp}.database.sqlite"
	(cd "$tree" && ulimit -t "$CPU_SECONDS" && WARP_NO_WINDOW=1 WARP_PORT=$SERVER_PORT timeout "$wall" "$WARP" "$1" </dev/null >"$out.txt" 2>"$out.err")
	local status=$? first verdict=FAIL
	[ $status -eq 124 ] && grep -aq "$SERVING" "$out.txt" && { echo "SRV  $1"; return; }
	[ $status -eq 0 ] && { echo "ok   $1"; return; }
	first=$(grep -av '^wasm-rt warning\|^warning: ' "$out.err" | grep -am1 . | tr '\t' ' ' | cut -c1-200)
	[ $status -eq 124 ] && { verdict=TIME; first="still running after ${wall} s"; }
	[ $status -eq $CPU_LIMIT_HIT ] && { verdict=TIME; first="used its ${CPU_SECONDS} s of CPU"; }
	grep -q "$EXPECTED_FAILURE" "$tree/$1" && verdict=EXP
	printf '%-4s %s: %s\n' "$verdict" "$1" "${first:-exit $status, nothing on stderr}"
}
export -f one wall_seconds
export tree results WARP CPU_SECONDS WALL_SECONDS CPU_LIMIT_HIT WINDOW_LIBRARIES SERVING SERVER_PORT EXPECTED_FAILURE

report=$results/report.txt
(cd "$tree" && find samples -name '*.warp' | sort) | xargs -P"$PARALLEL" -I{} bash -c 'one "$@"' _ {} | sort -k2 >"$report"
# a reason without its position and quoted texts: one cause reads alike in every sample and from run to run
sed 's/"[^"]*"/"…"/g; s/ ([0-9]*)//g; s/ at samples\/[^ ]*//; s/ (no owner)//' "$report" >"$tree/test/samples.txt"
results_header "$tree/test/samples.txt" "$(git -C "$tree" rev-parse --short=10 HEAD)" 'ok|SRV'
echo "samples: $(grep -c '^ok\|^SRV' "$report") of $(wc -l <"$report" | tr -d ' ') run, $(grep -c '^WIN' "$report") need a window, $(grep -c '^SRV' "$report") serve, $(grep -c '^EXP' "$report") fail on purpose, $(grep -c '^TIME' "$report") hit a limit ($WARP, $(git -C "$tree" rev-parse --short=10 HEAD)) -> $tree/test/samples.txt"
grep -aE '^(FAIL|TIME)' "$tree/test/samples.txt" | sed 's/^[A-Z]* *[^:]*: //' | sort | uniq -c | sort -rn
