#!/bin/bash
# The suite through the warp compiler (card warp-pipeline-suite, notes/self_hosting.md): every test with
# WARP_SELF_HOSTED=1 (src/compiler/self_hosted.rs: parser.warp parses, the pass twins of lib/compiler/passes.warp
# replace their Rust passes, the rest stays Rust), the twin tests (parser::test_self_*) left out. Prints
# "self-hosted: N passed / M" and the failures grouped by cause, the most frequent first.
#   scripts/self-hosted-suite.sh [<tip>]               on the cloud lane: the pushed tip (default HEAD) plus a throwaway
#                                                      commit switching self-hosting on (.cargo/config.toml [env]), one
#                                                      job per TEST_GROUPS entry at once, each on its own branch
#                                                      SCRATCH_PREFIX/<tip>-<group>, deleted after the run
#   scripts/self-hosted-suite.sh --local <filters…>    here, through tests/queue.sh
#   scripts/self-hosted-suite.sh --rank <log[.gz]>…    the count and the ranking of test logs
set -uo pipefail
SWITCH=WARP_SELF_HOSTED
SKIPPED=parser::test_self_       # the twins compare warp with Rust themselves
SCRATCH_PREFIX=self-hosted       # the throwaway branches
CONFIG=.cargo/config.toml
# the suite's speed as test.sh runs it (cloud-test.sh --full): the hosted parser runs in the tests' warp-lang
SPEED_PROFILE=$'\n[profile.test.package.warp-lang]\nopt-level = 2'
# The suite in parts of ~2000 tests that run at once: self-hosted, the whole suite took longer than the lane's
# 2400 s limit (6020 tests of ~6300), and a killed run prints no failure messages
TEST_GROUPS=(
	"control:: functions:: ffi::"
	"types:: lists:: modules:: node:: numbers:: operators::"
	"parser:: programs:: scope:: sweeps:: text:: wasm:: web:: welcoming::"
)
REPORTS=/Users/me/dev/angles/warp/data/self-hosted
CLOUD_RESULTS=/Users/me/dev/angles/warp/data/cloud
SHOWN_CAUSES=40
CAUSE_WIDTH=110

fail() { echo "self-hosted-suite: $*" >&2; exit 1; }

# The passed/total count from the test lines and each failure's cause: the panic message, for an assertion its left
# side (the value the self-hosted run gave), digits as #, so the same cause in different tests counts once per test
rank() {
	local log
	for log in "$@"; do
		if [[ "$log" == *.gz ]]; then gunzip -c "$log"; else cat "$log"; fi
	done | awk -v shown=$SHOWN_CAUSES -v width=$CAUSE_WIDTH '
		/^test [^ ]+ \.\.\. ok$/ { passed++ }
		/^test [^ ]+ \.\.\. FAILED$/ { failed++ }
		/^cloud-test: KILLED/ { killed++ }
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
			all = passed + failed
			printf "self-hosted: %d passed / %d (%.1f%%)\n", passed, all, all ? 100 * passed / all : 0
			if (killed) printf "%d killed runs: their unfinished tests are not counted, their failures have no cause\n", killed
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

# The tip with the switch on: a commit of its own per group, so each job has its own run name
switched_on() {
	local tip=$1 group=$2 index blob commit
	index=$(mktemp "${TMPDIR:-/tmp}/self-hosted-index.XXXXXX")
	GIT_INDEX_FILE=$index git read-tree "$tip"
	blob=$({ git show "$tip:$CONFIG"; echo "$SWITCH = \"1\""; echo "$SPEED_PROFILE"; } | git hash-object -w --stdin)
	GIT_INDEX_FILE=$index git update-index --cacheinfo "100644,$blob,$CONFIG"
	commit=$(git commit-tree "$(GIT_INDEX_FILE=$index git write-tree)" -p "$tip" -m "self-hosted suite run of $tip: $group")
	rm -f "$index"
	echo "$commit"
}

cloud_run() {
	local tip group number branch commit logs=() branches=() commits=()
	tip=$(git rev-parse "${1:-HEAD}") || fail "no tip ${1:-HEAD}"
	git branch -r --contains "$tip" | grep -q . || fail "tip $tip is not pushed"
	[ "$(git show "$tip:$CONFIG" | grep '^\[' | tail -1)" = "[env]" ] || fail "$CONFIG of $tip does not end with its [env] table"
	for number in "${!TEST_GROUPS[@]}"; do
		group=${TEST_GROUPS[$number]}
		branch=$SCRATCH_PREFIX/${tip:0:8}-$number
		commit=$(switched_on "$tip" "$group")
		git push -q -f origin "$commit:refs/heads/$branch" || fail "push of $branch failed"
		# shellcheck disable=SC2086 # the group is a list of filters
		scripts/cloud-test.sh --no-wait "$branch" "$commit" --all-features -- $group --skip "$SKIPPED" > /dev/null
		branches+=("$branch") commits+=("$commit")
	done
	for number in "${!branches[@]}"; do
		branch=${branches[$number]} commit=${commits[$number]}
		scripts/cloud-test.sh --result "$branch" "$commit" > /dev/null
		git push -q origin --delete "$branch"
		logs+=("$CLOUD_RESULTS/$(printf %s "$branch" | tr -c 'A-Za-z0-9._-' '-')-${commit:0:8}/log.txt.gz")
	done
	mkdir -p "$REPORTS"
	rank "${logs[@]}" | tee "$REPORTS/${tip:0:8}.txt"
	echo "(report: $REPORTS/${tip:0:8}.txt, logs: ${logs[*]})"
}

case "${1:-}" in
	--rank) shift; rank "$@" ;;
	--local) shift; local_run "$@" ;;
	--*) sed -n '2,11p' "$0"; exit 2 ;;
	*) cloud_run "$@" ;;
esac
