#!/bin/bash
# The suite through the warp compiler (card warp-pipeline-suite, notes/self_hosting.md): every test with
# WARP_SELF_HOSTED=1 (src/compiler/self_hosted.rs: parser.warp parses, the pass twins of lib/compiler/passes.warp
# replace their Rust passes, the rest stays Rust), the twin tests (parser::test_self_*) left out. Prints
# "self-hosted: N passed / M" and the failures grouped by cause, the most frequent first.
#   scripts/self-hosted-suite.sh [<tip>]               on the cloud lane: the pushed tip (default HEAD) plus a throwaway
#                                                      commit switching self-hosting on (.cargo/config.toml [env]),
#                                                      pushed as SCRATCH_PREFIX/<tip>, deleted after the run
#   scripts/self-hosted-suite.sh --local <filters…>    here, through tests/queue.sh
#   scripts/self-hosted-suite.sh --rank <log[.gz]>     the count and the ranking of a test log
set -uo pipefail
SWITCH=WARP_SELF_HOSTED
SKIPPED=parser::test_self_       # the twins compare warp with Rust themselves
SCRATCH_PREFIX=self-hosted       # the throwaway branches
CONFIG=.cargo/config.toml
# the suite's speed as test.sh runs it (cloud-test.sh --full): the hosted parser runs in the tests' warp-lang
SPEED_PROFILE=$'\n[profile.test.package.warp-lang]\nopt-level = 2'
REPORTS=/Users/me/dev/angles/warp/data/self-hosted
SHOWN_CAUSES=40
CAUSE_WIDTH=110

fail() { echo "self-hosted-suite: $*" >&2; exit 1; }

# The passed/total count and each failure's cause: the panic message, for an assertion its left side (the value the
# self-hosted run gave), digits as #, so the same cause in different tests counts once per test
rank() {
	local log=$1 reader=cat
	[[ "$log" == *.gz ]] && reader="gunzip -c"
	$reader "$log" | awk -v shown=$SHOWN_CAUSES -v width=$CAUSE_WIDTH '
		/^test result:/ { for (i = 1; i <= NF; i++) if ($(i) ~ /^(passed|failed)/) { key = $(i); sub(/[;,]/, "", key); total[key] += $(i - 1) } }
		/^---- [^ ]+ stdout ----$/ { current = $2; state = 1; next }
		state == 1 && / panicked at / { state = 2; next }
		state == 2 { message = $0; state = (message ~ /^assertion/) ? 3 : 0; if (!state) count_cause(message); next }
		state == 3 { sub(/^ *left: */, ""); count_cause($0); state = 0 }
		function count_cause(cause) {
			gsub(/[0-9]+/, "#", cause)
			cause = substr(cause, 1, width)
			if (!(cause in causes)) example[cause] = current
			causes[cause]++
		}
		END {
			all = total["passed"] + total["failed"]
			printf "self-hosted: %d passed / %d (%.1f%%)\n", total["passed"], all, all ? 100 * total["passed"] / all : 0
			printf "failures by cause (count, cause, one test):\n"
			sorter = "sort -rn | head -" shown
			for (cause in causes) printf "%6d  %s  [%s]\n", causes[cause], cause, example[cause] | sorter
		}'
}

local_run() {
	mkdir -p "$REPORTS"
	local log=$REPORTS/local.log
	env "$SWITCH=1" tests/queue.sh -- "$@" --skip "$SKIPPED" > "$log" 2>&1
	rank "$log"
	echo "(log: $log)"
}

cloud_run() {
	local tip commit index blob branch run_name
	tip=$(git rev-parse "${1:-HEAD}") || fail "no tip ${1:-HEAD}"
	git branch -r --contains "$tip" | grep -q . || fail "tip $tip is not pushed"
	[ "$(git show "$tip:$CONFIG" | grep '^\[' | tail -1)" = "[env]" ] || fail "$CONFIG of $tip does not end with its [env] table"
	index=$(mktemp "${TMPDIR:-/tmp}/self-hosted-index.XXXXXX")
	GIT_INDEX_FILE=$index git read-tree "$tip"
	blob=$({ git show "$tip:$CONFIG"; echo "$SWITCH = \"1\""; echo "$SPEED_PROFILE"; } | git hash-object -w --stdin)
	GIT_INDEX_FILE=$index git update-index --cacheinfo "100644,$blob,$CONFIG"
	commit=$(git commit-tree "$(GIT_INDEX_FILE=$index git write-tree)" -p "$tip" -m "self-hosted suite run of $tip")
	rm -f "$index"
	branch=$SCRATCH_PREFIX/${tip:0:8}
	git push -q -f origin "$commit:refs/heads/$branch" || fail "push of $branch failed"
	scripts/cloud-test.sh "$branch" "$commit" --all-features -- --skip "$SKIPPED" > /dev/null
	git push -q origin --delete "$branch"
	run_name="$(printf %s "$branch" | tr -c 'A-Za-z0-9._-' '-')-${commit:0:8}"
	mkdir -p "$REPORTS"
	rank "/Users/me/dev/angles/warp/data/cloud/$run_name/log.txt.gz" | tee "$REPORTS/${tip:0:8}.txt"
	echo "(report: $REPORTS/${tip:0:8}.txt, log: /Users/me/dev/angles/warp/data/cloud/$run_name/log.txt.gz)"
}

case "${1:-}" in
	--rank) rank "$2" ;;
	--local) shift; local_run "$@" ;;
	--*) sed -n '2,11p' "$0"; exit 2 ;;
	*) cloud_run "$@" ;;
esac
