#!/usr/bin/env bash
# scripts/baseline.sh <test paths…>: the stored baseline lines of those tests, so a branch never reruns them on main.
# scripts/baseline.sh --regressions <results file>: the tests ✓ in the baseline that are ✗ in that file (a branch's own
# `warp test <paths>` output), one a line; exit 1 when there are any.
# The baseline is the full suite's results stored in the record of main's installed compiler
# (<store>/<hash>/warp.record/results.txt, written by scripts/full-suite.sh); without one, the newest stored results,
# and without any, test/results.txt at its
# commit, said loudly, since that is tied to a commit, not to the compiler a branch was built on.
set -uo pipefail
source "$(dirname "$0")/lib/platform.sh"
WARP=${WARP:-$warp_binary}
STORE=$warp_store
FALLBACK=$(git rev-parse --show-toplevel)/test/results.txt

compiler_hash=$(cut -d' ' -f1 "$(dirname "$(readlink -f "$WARP")")/warp.wasm.inputs" 2>/dev/null)
baseline=$STORE/$compiler_hash/warp.record/results.txt
if [ -n "$compiler_hash" ] && [ -f "$baseline" ]; then
	echo "baseline: compiler $compiler_hash (full suite at $(cat "${baseline%.txt}.commit" 2>/dev/null))" >&2
else
	# installs outpace the hour-long full suite, so main's compiler often has no results yet: the newest stored ones
	newest=$(ls -t "$STORE"/*/warp.record/results.txt 2>/dev/null | head -1)
	if [ -n "$newest" ]; then
		echo "baseline: no stored results for main's compiler ${compiler_hash:-?}; using the newest, compiler $(basename "$(dirname "$(dirname "$newest")")") (full suite at $(cat "${newest%.txt}.commit" 2>/dev/null)): a red line there may be fixed since" >&2
		baseline=$newest
	else
		echo "baseline: no stored results in $STORE; using $FALLBACK (a commit's, not this compiler's)" >&2
		baseline=$FALLBACK
	fi
fi

# a result line without its leading mark, and a failure's without what it got: the test's place, code and expectation,
# the same in both files
case_of() { sed -e 's/^[^ ]* //' -e 's/: got .*$//'; }

if [ "${1:-}" = --regressions ]; then
	[ -f "${2:-}" ] || { echo "usage: $0 --regressions <results file>" >&2; exit 2; }
	regressions=$(comm -12 <(grep '^✓' "$baseline" | case_of | sort) <(grep '^✗' "$2" | case_of | sort))
	[ -z "$regressions" ] && { echo "baseline: no regressions" >&2; exit 0; }
	echo "$regressions"
	exit 1
fi
[ $# -gt 0 ] || { echo "usage: $0 <test paths…> | --regressions <results file>" >&2; exit 2; }
for path in "$@"; do grep -F " ${path%/}" "$baseline"; done
