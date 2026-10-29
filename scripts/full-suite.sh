#!/usr/bin/env bash
# scripts/full-suite.sh [--commit] [<tree>, default this checkout]: the full suite, `warp test --each test` of the
# tree, into its tracked test/results.txt (a ✓ or ✗ line per test, in file order, then the `m of n failed` summary);
# --commit commits and pushes it when it changed ("chore: full suite <summary> at <commit>"), so git diff and git log -p
# of it name what went green or red.
# One at a time on this machine: a second call while one runs says so and exits 0 (two full suites at once doubled the
# load to 200 and wrote one results file, 2026-10-29). The results land only when the run ended with its summary; a run
# that died leaves test/results.txt as it was, says why (its last error lines) and keeps its output in data/full-suite/.
# Run it in the background: it takes about an hour.
set -uo pipefail
WARP=${WARP:-/Users/me/dev/bin/warp}
LOCK=$HOME/.cache/warp/full-suite.lock
SUMMARY='failed in [0-9]* files\|tests passed in [0-9]* files'

commit=0
[ "${1:-}" = --commit ] && { commit=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
report=test/results.txt
scratch=$tree/data/full-suite
mkdir -p "$scratch" "$(dirname "$LOCK")"

# the lock holds the runner's pid and tree; a dead runner's is taken over
if ! mkdir "$LOCK" 2>/dev/null; then
	holder=$(cat "$LOCK/pid" 2>/dev/null)
	if [ -n "$holder" ] && kill -0 "$holder" 2>/dev/null; then
		echo "full-suite: already running (pid $holder, $(cat "$LOCK/tree" 2>/dev/null), since $(cat "$LOCK/since" 2>/dev/null)): its results land in test/results.txt"
		exit 0
	fi
	rm -rf "$LOCK" && mkdir "$LOCK" || exit 1
fi
echo $$ >"$LOCK/pid"
echo "$tree" >"$LOCK/tree"
date '+%H:%M' >"$LOCK/since"
trap 'rm -rf "$LOCK"' EXIT

cd "$tree" || exit 1
tip=$(git rev-parse --short=10 HEAD)
WARP_NO_WINDOW=1 WARP_HINTS=0 "$WARP" test --each test >"$scratch/results.txt" 2>"$scratch/errors.txt"
summary=$(tail -1 "$scratch/results.txt" | grep "$SUMMARY")
if [ -z "$summary" ]; then
	echo "full-suite: the run at $tip died without its summary; test/results.txt kept. Last errors:" >&2
	tail -3 "$scratch/errors.txt" >&2
	exit 1
fi
mv "$scratch/results.txt" "$report"
echo "full-suite: $summary at $tip"
((commit)) || exit 0
git add -- "$report" && git diff --cached --quiet -- "$report" && exit 0
git commit -q -m "chore: full suite $summary at $tip" -- "$report" && git pull -q --rebase --autostash && git push -q
