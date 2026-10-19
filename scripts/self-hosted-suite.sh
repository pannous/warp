#!/bin/bash
# The suite through the warp compiler (card warp-pipeline-suite, notes/self_hosting.md): every test with
# WARP_SELF_HOSTED=front (src/compiler/self_hosted.rs: parser.warp parses, the pass twins of lib/compiler/passes.warp
# replace their Rust passes, the rest stays Rust; =1 adds emitter.warp), the twin tests (parser::test_self_*) left out. Prints
# "self-hosted: N passed / M" and the failures grouped by cause, the most frequent first.
#   scripts/self-hosted-suite.sh [<tip>]               on the cloud lane: the pushed tip (default HEAD) plus a throwaway
#                                                      commit switching self-hosting on (.cargo/config.toml [env]), one
#                                                      job per TEST_GROUPS entry at once, each on its own branch
#                                                      SCRATCH_PREFIX/<tip>-<group>, deleted after the run
#   scripts/self-hosted-suite.sh --stage <name>=<VAR>[=<value>]… [<tip>]   the same with a later stage's switch set too
#                                                      (value 1 by default; WARP_SELF_HOSTED's own replaces "front"),
#                                                      counted apart as run <tip>+<name>; notes/self_hosting.md
#                                                      "Back-end focus": front end only, + emitter (--stage
#                                                      emitter=WARP_SELF_HOSTED=1), + analyzer (--stage
#                                                      analyzer=WARP_SELF_ANALYSIS: analyzer.warp checks each program)
#   scripts/self-hosted-suite.sh --local <filters…>    here, through tests/queue.sh (after --stage …: with that stage,
#                                                      e.g. --stage emitter=WARP_SELF_HOSTED=1 --local lists::)
#   scripts/self-hosted-suite.sh --rank <log[.gz]>…    the count and the ranking of test logs
#   scripts/self-hosted-suite.sh --tests <regex> <log[.gz]>…   each failed test whose cause matches, with its cause
set -uo pipefail
SWITCH=WARP_SELF_HOSTED
SWITCHES=("$SWITCH=front")       # VAR=value lines of .cargo/config.toml [env]: the front end alone, unless --stage adds
STAGE=""                         # the later stages switched on too (--stage), as the run's name suffix
SKIPPED=parser::test_self_       # the twins compare warp with Rust themselves
SCRATCH_PREFIX=self-hosted       # the throwaway branches
CONFIG=.cargo/config.toml
# the suite's speed as test.sh runs it (cloud-test.sh --full): the hosted parser runs in the tests' warp-lang
SPEED_PROFILE=$'\n[profile.test.package.warp-lang]\nopt-level = 2'
# The suite in parts of ~1600 tests that run at once: self-hosted, the whole suite took longer than the lane's
# 2400 s limit (6020 tests of ~6300), and a killed run prints no failure messages; the third part of ~2000 outgrew it
# too (8f8d6dc8). Filters match anywhere in a test's path (lists:: also takes control::test_job_lists), so a test
# may run in two parts: rank counts each test once.
TEST_GROUPS=(
	"control:: functions:: ffi::"
	"types:: lists:: modules:: node:: numbers:: operators::"
	"parser:: programs:: scope:: sweeps::"
	"text:: wasm:: web:: welcoming::"
)
REPORTS=/Users/me/dev/angles/warp/data/self-hosted
COUNTS=/Users/me/dev/angles/warp/data/self_hosting/suite_counts.txt   # the count over time, one line per cloud run
CLOUD_RESULTS=/Users/me/dev/angles/warp/data/cloud
SHOWN_CAUSES=40
FAMILY_LEAST=10                  # a family of fewer failures is only listed by its causes
CAUSE_WIDTH=110

fail() { echo "self-hosted-suite: $*" >&2; exit 1; }

# The passed/total count from the test lines and each failure's cause: the panic message, for an assertion its left
# side (the value the self-hosted run gave), digits as #, so the same cause in different tests counts once per test
# With LISTED set (a regex), only the failed tests whose cause matches, each with its cause
rank() {
	local log
	for log in "$@"; do
		if [[ "$log" == *.gz ]]; then gunzip -c "$log"; else cat "$log"; fi
	done | LC_ALL=C awk -v listed="${LISTED:-}" -v shown=$SHOWN_CAUSES -v width=$CAUSE_WIDTH -v family_least=$FAMILY_LEAST '
		match($0, /(^|[^ ])test [^ ]+ \.\.\. (ok|FAILED)$/) { result($0) }
		/^cloud-test: KILLED/ { killed++ }
		/^---- [^ ]+ stdout ----$/ { current = $2; state = 1; next }
		state == 1 && / panicked at / { state = (current in caused) ? 0 : 2; caused[current]; next }
		state == 2 { message = $0; state = (message ~ /^assertion/) ? 3 : 0; if (!state) count_cause(message); next }
		state == 3 { sub(/^ *left: */, ""); count_cause($0); state = 0 }
		# a test that ran in two parts counts once, failed if it failed in either
		function result(line,   name) {
			sub(/^.*test /, "", line); name = line; sub(/ .*$/, "", name)
			if (line ~ /FAILED$/) outcome[name] = "FAILED"
			else if (!(name in outcome)) outcome[name] = "ok"
		}
		# the message itself: an error value or a text of one unwrapped, its quotes unescaped
		function count_cause(cause) {
			gsub(/\\"/, "\"", cause)
			sub(/^"/, "", cause); sub(/"$/, "", cause)
			sub(/^Error\("/, "", cause); sub(/"\)$/, "", cause)
			gsub(/[0-9]+/, "#", cause)
			cause = substr(cause, 1, width)
			if (listed != "") { if (cause ~ listed) print current "\t" cause; return }
			if (!(cause in causes)) example[cause] = current
			causes[cause]++
			family = cause
			gsub(/\x27[^\x27]*\x27|`[^`]*`|: [^ ]+$/, " …", family)
			families[family]++
		}
		END {
			if (listed != "") exit
			for (name in outcome) if (outcome[name] == "ok") passed++; else failed++
			all = passed + failed
			printf "self-hosted: %d passed / %d (%.1f%%)\n", passed, all, all ? 100 * passed / all : 0
			if (killed) printf "%d killed runs: their unfinished tests are not counted, their failures have no cause\n", killed
			sorter = "LC_ALL=C sort -rn | head -" shown
			printf "failure families (quoted parts as …):\n"
			for (family in families) if (families[family] >= family_least) printf "%6d  %s\n", families[family], family | sorter
			close(sorter)
			printf "failures by cause (count, cause, one test):\n"
			for (cause in causes) printf "%6d  %s  [%s]\n", causes[cause], cause, example[cause] | sorter
		}'
}

local_run() {
	mkdir -p "$REPORTS"
	local log=$REPORTS/local.log
	env "${SWITCHES[@]}" tests/queue.sh -- "$@" --skip "$SKIPPED" > "$log" 2>&1
	rank "$log"
	echo "(log: $log)"
}

# The failed tests of a run's logs, sorted
failed_tests() {
	local log
	for log in "$@"; do gunzip -c "$log"; done | sed -n 's/^.*test \([^ ]*\) \.\.\. FAILED$/\1/p' | LC_ALL=C sort -u
}

# --stage <name>=<VAR>[=<value>]: the run's name grows by the stage, VAR is set (again) to the value
set_switch() {
	local variable=${1#*=} value=1 index
	[[ "$variable" == *=* ]] && value=${variable#*=} variable=${variable%%=*}
	STAGE=${STAGE:+$STAGE+}${1%%=*}
	for index in "${!SWITCHES[@]}"; do
		[ "${SWITCHES[$index]%%=*}" = "$variable" ] && { SWITCHES[index]="$variable=$value"; return; }
	done
	SWITCHES+=("$variable=$value")
}

# A run's name as its cloud results folder spells it
folder_name() { printf %s "$1" | tr -c 'A-Za-z0-9._-' '-'; }

# The tests failing now that passed in the last recorded run (a regression check), from the logs both runs left
newly_failed() {
	local previous=$1; shift
	local previous_logs=("$CLOUD_RESULTS/$(folder_name "$SCRATCH_PREFIX/$previous")"-[0-9]-*/log.txt.gz)
	[ -e "${previous_logs[0]}" ] || return 0
	echo "newly failing since $previous:"
	LC_ALL=C comm -13 <(failed_tests "${previous_logs[@]}") <(failed_tests "$@")
}

# The tip with the switch on: a commit of its own per group, so each job has its own run name
switched_on() {
	local tip=$1 group=$2 index blob commit
	index=$(mktemp "${TMPDIR:-/tmp}/self-hosted-index.XXXXXX")
	GIT_INDEX_FILE=$index git read-tree "$tip"
	blob=$({ git show "$tip:$CONFIG"; printf '%s\n' "${SWITCHES[@]}" | sed 's/^\([^=]*\)=\(.*\)$/\1 = "\2"/'; echo "$SPEED_PROFILE"; } |
		git hash-object -w --stdin)
	GIT_INDEX_FILE=$index git update-index --cacheinfo "100644,$blob,$CONFIG"
	commit=$(git commit-tree "$(GIT_INDEX_FILE=$index git write-tree)" -p "$tip" -m "self-hosted suite run of $tip: $group")
	rm -f "$index"
	echo "$commit"
}

cloud_run() {
	local tip run group number branch commit logs=() branches=() commits=()
	tip=$(git rev-parse "${1:-HEAD}") || fail "no tip ${1:-HEAD}"
	run=${tip:0:8}${STAGE:++$STAGE}
	git branch -r --contains "$tip" | grep -q . || fail "tip $tip is not pushed"
	[ "$(git show "$tip:$CONFIG" | grep '^\[' | tail -1)" = "[env]" ] || fail "$CONFIG of $tip does not end with its [env] table"
	for number in "${!TEST_GROUPS[@]}"; do
		group=${TEST_GROUPS[$number]}
		branch=$SCRATCH_PREFIX/$run-$number
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
		logs+=("$CLOUD_RESULTS/$(folder_name "$branch")-${commit:0:8}/log.txt.gz")
	done
	mkdir -p "$REPORTS"
	local report=$REPORTS/$run.txt previous
	rank "${logs[@]}" | tee "$report"
	mkdir -p "$(dirname "$COUNTS")"
	# against the last complete run of the same stages: a line noting "incomplete" lacks the tests of its killed part
	previous=$(grep -v -e '^#' -e incomplete "$COUNTS" 2>/dev/null | awk -v stage="$STAGE" '
		{ plus = index($2, "+"); if ((plus ? substr($2, plus + 1) : "") == stage) last = $2 } END { print last }')
	[ -n "$previous" ] && newly_failed "$previous" "${logs[@]}" | tee -a "$report"
	printf '%s  %s  %s  %s\n' "$(date +%F)" "$run" "$(head -1 "$report" | sed 's/^self-hosted: //; s/ passed//')" \
		"data/self-hosted/$run.txt" >> "$COUNTS"
	echo "(report: $report, logs: ${logs[*]})"
}

case "${1:-}" in
	--rank) shift; rank "$@" ;;
	--tests) shift; LISTED=$1; shift; rank "$@" ;;
	--stage)
		while [ "${1:-}" = --stage ]; do
			set_switch "$2"
			shift 2
		done
		if [ "${1:-}" = --local ]; then shift; local_run "$@"; else cloud_run "$@"; fi ;;
	--local) shift; local_run "$@" ;;
	--*) sed -n '2,18p' "$0"; exit 2 ;;
	*) cloud_run "$@" ;;
esac
