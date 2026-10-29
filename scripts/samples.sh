#!/usr/bin/env bash
# scripts/samples.sh [<tree>, default this checkout]: how many of the tree's samples/ run, the gap to 2.0 (decision
# release-1-5-tonight). Each samples/**/*.warp runs from the tree's root through the installed warp (WARP_NO_WINDOW,
# 30 s each, 4 at a time); one fails when the native warp exits other than 0, its error the first line it wrote to
# stderr. Writes test/samples.txt, tracked (status and sample, one a line: after-push commits it when it changed, so
# git diff and git log -p of it name the samples that changed), data/samples/report.txt (each failure's first error
# line too) and each run's output to data/samples/out/ (<sample>.txt, <sample>.err); prints the count and the failures
# grouped by their first error.
set -uo pipefail
WARP=${WARP:-/Users/me/dev/bin/warp}
SECONDS_EACH=30
PARALLEL=4
# a sample calling a C window library opens its window whatever WARP_NO_WINDOW says: not run, counted apart (until
# FFI windows honour WARP_NO_WINDOW, warp-native-harness)
WINDOW_LIBRARIES='raylib|SDL|glfw'
# a server sample serves until its timeout once it printed this (lib/web.warp serve): served, counted apart; each on a
# port the system chooses (SERVER_PORT 0), so samples run at once, other runs and tests never collide
SERVING='^serving http://'
SERVER_PORT=0

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
	(cd "$tree" && WARP_NO_WINDOW=1 WARP_PORT=$SERVER_PORT timeout "$SECONDS_EACH" "$WARP" "$1" </dev/null >"$out.txt" 2>"$out.err")
	local status=$? first
	[ $status -eq 124 ] && grep -aq "$SERVING" "$out.txt" && { echo "SRV  $1"; return; }
	[ $status -eq 0 ] && { echo "ok   $1"; return; }
	first=$(grep -av '^wasm-rt warning\|^warning: ' "$out.err" | grep -am1 . | tr '\t' ' ' | cut -c1-200)
	[ $status -eq 124 ] && first="timed out after ${SECONDS_EACH} s"
	echo "FAIL $1: ${first:-exit $status, nothing on stderr}"
}
export -f one
export tree results WARP SECONDS_EACH WINDOW_LIBRARIES SERVING SERVER_PORT

report=$results/report.txt
(cd "$tree" && find samples -name '*.warp' | sort) | xargs -P"$PARALLEL" -I{} bash -c 'one "$@"' _ {} | sort -k2 >"$report"
cut -d: -f1 "$report" >"$tree/test/samples.txt"
echo "samples: $(grep -c '^ok\|^SRV' "$report") of $(wc -l <"$report" | tr -d ' ') run, $(grep -c '^WIN' "$report") need a window, $(grep -c '^SRV' "$report") serve ($WARP, $(git -C "$tree" rev-parse --short=10 HEAD)) -> $tree/test/samples.txt"
# the failures by first error, its position and quotes left out so one cause groups
grep -a '^FAIL' "$report" | sed 's/^FAIL [^:]*: //; s/"[^"]*"/"…"/g; s/ at samples\/[^ ]*//; s/ (no owner)//' | sort | uniq -c | sort -rn
