#!/bin/bash
# The cloud test lane (card cloud-test-lane, notes/cloud_development.md "Cloud test lane"): a branch's tests run in a
# `claude --cloud` session, a Linux VM of its own paid by the Cloud session credit, instead of the Mac's tests/queue.sh.
#   scripts/cloud-test.sh [--card <card>] [--no-wait] [--on-main] <branch> <tip> [filters…|--full]
#       filters as for tests/queue.sh (`-- x y` or plain `x y`; `--test tests -- x` passes target flags to cargo);
#       --full: the whole suite as test.sh runs it (--all-features, warp-lang at opt-level 2)
#       waits for the result (run it in the background), prints it, keeps it in data/cloud/<run>/ and, with --card,
#       adds it to the card (`todo note`); --no-wait only launches: fetch later with --result;
#       --on-main tests the tip merged into the current origin/main, as the Integrator's gate will
#   scripts/cloud-test.sh --result [--card <card>] <branch> <tip>     wait for and collect a launched run's result
#   scripts/cloud-test.sh --summarize <cargo test log> [environmental list]    the result's summary of any log
# The cloud session runs this same script from origin/main (the tip may predate it) with --run-here, which checks out
# the tip, builds, tests with --no-fail-fast and pushes result.md + log.txt.gz to the branch cloud-results/<run>.
# Failures of tests that need the Mac (scripts/cloud-test-environmental.txt) are listed apart and don't make it red.
# CLOUD_TEST_FAST=1: fast mode (Opus with faster output, burns the credit ~2x faster; off by default).
# CLOUD_TEST_MODEL=<model>: the session's model (default: the cloud environment's).
set -uo pipefail

REPO=pannous/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
RESULTS_PREFIX=cloud-results
ENVIRONMENTAL_LIST=scripts/cloud-test-environmental.txt
MAX_PARALLEL_RUNS=3            # cloud sessions at once (they share the account's limits, notes/cloud_development.md)
PENDING_HOURS=3                # a launch older than this without a result no longer counts as running
POLL_SECONDS=60                # GitHub API quota is shared by every session: ls-remote once a minute
WAIT_HOURS=4
URL_WAIT_SECONDS=90
HERE_WAIT_SECONDS=540          # one --run-here call blocks at most this long, under the cloud Bash tool's 10 min
HERE_WORK=${TMPDIR:-/tmp}/cloud-test
FULL_FLAGS=(--all-features --config "profile.test.package.warp-lang.opt-level=2")
USAGE_SCRIPT=$HOME/dev/bin/claude-usage.sh

fail() { echo "cloud-test: $*" >&2; exit 1; }
run_name() { echo "$(printf %s "$1" | tr -c 'A-Za-z0-9._-' '-')-${2:0:8}"; }
credit_used() { "$USAGE_SCRIPT" 2>/dev/null | sed -n 's/^iguana_necktie: \$\([0-9.]*\) .*/\1/p'; }

# ---------------------------------------------------------------- the cloud side

# The run itself, detached from the session's Bash tool: checkout, build, test, summarize, push
run_tests_here() {
	local on_main=""
	[ "$1" = "--on-main" ] && on_main=1 && shift
	local branch=$1 tip=$2; shift 2
	local name started build_seconds test_seconds checkout lock_matches=""
	name=$(run_name "$branch" "$tip")
	started=$SECONDS
	local branch_ref
	for branch_ref in "$branch" main vendor; do
		git fetch -q origin "+refs/heads/$branch_ref:refs/remotes/origin/$branch_ref" 2>&1 | tail -3
	done
	git cat-file -e "$tip^{commit}" 2>/dev/null || git fetch -q origin "$tip" || fail "tip $tip not found on origin"
	checkout=$HERE_WORK/tree
	rm -rf "$checkout" && git worktree prune && git worktree add -q --detach "$checkout" "$tip" || fail "checkout of $tip failed"
	git show "origin/main:$ENVIRONMENTAL_LIST" > "$HERE_WORK/environmental.txt" 2>/dev/null
	cd "$checkout" || fail "no checkout"
	local tested=$tip
	if [ -n "$on_main" ]; then
		git -c user.name=pannous -c user.email=info@pannous.com merge -q --no-edit origin/main || fail "$tip does not merge cleanly into origin/main"
		tested="$tip merged into origin/main $(git rev-parse origin/main)"
	fi
	rustup target add wasm32-wasip1 >/dev/null 2>&1 # the package/uniscript tests build wasm32-wasip1 guests
	local source_flags=(--config net.offline=false)
	if git diff --quiet origin/vendor HEAD -- Cargo.lock 2>/dev/null; then
		lock_matches=1
		git archive origin/vendor vendor | tar -x
		source_flags=(--offline --config 'source.crates-io.replace-with="vendored-sources"' --config 'source.vendored-sources.directory="vendor"')
	fi
	local test_args=() filters=() past_separator=""
	if [ "${1:-}" = "--full" ]; then
		test_args=("${FULL_FLAGS[@]}")
	else
		for argument in "$@"; do
			if [ "$argument" = "--" ]; then past_separator=1
			elif [ -n "$past_separator" ] || [[ "$argument" != -* ]]; then filters+=("$argument")
			else test_args+=("$argument"); fi
		done
	fi
	export WARP_NO_WINDOW=1 WARP_CRASH_CARDS=0 CARGO_TERM_COLOR=never
	local build_start=$SECONDS
	cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast --no-run > "$HERE_WORK/log.txt" 2>&1
	local build_status=$?
	build_seconds=$((SECONDS - build_start))
	local test_start=$SECONDS
	[ $build_status -eq 0 ] && cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast -- "${filters[@]}" >> "$HERE_WORK/log.txt" 2>&1
	test_seconds=$((SECONDS - test_start))
	summarize "$HERE_WORK/log.txt" "$HERE_WORK/environmental.txt" > "$HERE_WORK/summary.txt"
	{
		echo "run: $name"
		echo "branch: $branch"
		echo "tip: $tested"
		echo "tests: ${*:-all}"
		echo "verdict: $(sed -n 's/^verdict: //p' "$HERE_WORK/summary.txt")"
		[ $build_status -ne 0 ] && echo "build: FAILED (log.txt.gz)"
		grep -v '^verdict: ' "$HERE_WORK/summary.txt"
		echo "wall: build ${build_seconds} s, tests ${test_seconds} s, total $((SECONDS - started)) s"
		echo "machine: $(nproc 2>/dev/null) cores, $(free -g 2>/dev/null | awk '/^Mem:/ { print $2 " GB" }'), $(rustc --version)"
		echo "crates: $([ -n "$lock_matches" ] && echo "vendor branch (offline)" || echo "crates.io (vendor branch's Cargo.lock differs)")"
		echo "finished: $(date -u +%FT%TZ)"
	} > "$HERE_WORK/result.md"
	gzip -kf "$HERE_WORK/log.txt"
	push_result "$name"
}

# Counts, failing tests with the first line of their panic, environmental ones apart; verdict green|red
summarize() {
	local log=$1 environmental=$2
	awk -v environmental="$environmental" '
		BEGIN { while ((getline line < environmental) > 0) { sub(/[ \t]*#.*/, "", line); if (line != "") patterns[++count] = line } }
		/^test result:/ { for (i = 1; i <= NF; i++) { if ($(i) ~ /^(passed|failed|ignored)/) { key = $(i); sub(/[;,]/, "", key); total[key] += $(i - 1) } } finished++ }
		/^ +(Running|Doc-tests) / { started++ }
		/^test [^ ]+ \.\.\. .*FAILED/ { failed[$2] = 1; order[++failures] = $2 }
		/^---- [^ ]+ stdout ----$/ { current = $2; want_panic = 1; next }
		want_panic && / panicked at / { getline message; panic[current] = message; want_panic = 0 }
		END {
			incomplete = started > finished ? started - finished : 0
			for (i = 1; i <= failures; i++) {
				name = order[i]; line = "  " name ": " (name in panic ? panic[name] : "(no panic line)")
				is_environmental = 0
				for (p = 1; p <= count; p++) if (name ~ patterns[p]) is_environmental = 1
				if (is_environmental) environmental_lines = environmental_lines line "\n"; else { real_lines = real_lines line "\n"; real++ }
			}
			real += incomplete
			printf "verdict: %s\n", (real == 0 && started > 0) ? "green" : "red"
			printf "passed: %d, failed: %d (%d environmental), ignored: %d\n", total["passed"], failures + incomplete, failures - (real - incomplete), total["ignored"]
			if (incomplete) printf "crashed test binaries (no result line): %d\n", incomplete
			if (real_lines != "") printf "failures:\n%s", real_lines
			if (environmental_lines != "") printf "environmental (Mac-only, %s):\n%s", "scripts/cloud-test-environmental.txt", environmental_lines
		}' "$log"
}

# result.md and log.txt.gz as the one commit of cloud-results/<run>, no history, nothing else checked out
push_result() {
	local name=$1 tree commit
	export GIT_INDEX_FILE=$HERE_WORK/result.index
	rm -f "$GIT_INDEX_FILE"
	git update-index --add --cacheinfo "100644,$(git hash-object -w "$HERE_WORK/result.md"),result.md"
	git update-index --add --cacheinfo "100644,$(git hash-object -w "$HERE_WORK/log.txt.gz"),log.txt.gz"
	tree=$(git write-tree)
	unset GIT_INDEX_FILE
	commit=$(git -c user.name=pannous -c user.email=info@pannous.com commit-tree "$tree" -m "cloud test result $name")
	git push -q -f origin "$commit:refs/heads/$RESULTS_PREFIX/$name" && echo "pushed $RESULTS_PREFIX/$name"
}

# What the cloud session calls, again and again until DONE: starts the run once, then waits for it a while
run_here() {
	mkdir -p "$HERE_WORK"
	if [ ! -e "$HERE_WORK/started" ]; then
		touch "$HERE_WORK/started"
		nohup bash "$0" --run-detached "$@" > "$HERE_WORK/runner.log" 2>&1 &
		echo "started the run (pid $!)"
	fi
	local waited=0
	while [ ! -e "$HERE_WORK/done" ] && [ $waited -lt $HERE_WAIT_SECONDS ]; do sleep 15; waited=$((waited + 15)); done
	if [ -e "$HERE_WORK/done" ]; then
		cat "$HERE_WORK/result.md" "$HERE_WORK/runner.log"
		echo "CLOUD TEST DONE"
	else
		echo "still running: $(grep -c '^test .* \.\.\. ' "$HERE_WORK/log.txt" 2>/dev/null || echo 0) tests reported so far;" \
			"last line: $(tail -1 "$HERE_WORK/log.txt" 2>/dev/null | cut -c1-160)"
		echo "CLOUD TEST RUNNING: call the same command again"
	fi
}

# ---------------------------------------------------------------- the Mac side

cloud_prompt() {
	local script_args=$1
	cat <<-EOF
	Repository pannous/warp: you are a test runner of the warp cloud test lane (scripts/cloud-test.sh). Do exactly this, nothing else:
	1. git fetch origin main && git show origin/main:scripts/cloud-test.sh > /tmp/cloud-test.sh
	2. bash /tmp/cloud-test.sh --run-here $script_args
	   Run it in the foreground with a 600000 ms timeout. Each call waits up to 9 minutes: while it prints "CLOUD TEST RUNNING",
	   call the same command again (it never starts a second run). Stop when it prints "CLOUD TEST DONE".
	3. Reply with the result lines it printed. Do not edit, commit or push anything yourself (the script pushes the result);
	   do not fix failing tests or investigate them.
	EOF
}

session_url() { grep -o 'https://claude.ai/code/session_[A-Za-z0-9_]*' "$1" | head -1; }

pending_runs() {
	local launch count=0
	for launch in "$MAIN_CHECKOUT"/data/cloud/*/launch.txt; do
		[ -e "$launch" ] || continue
		[ -e "$(dirname "$launch")/result.md" ] && continue
		[ -n "$(find "$launch" -mmin -$((PENDING_HOURS * 60)))" ] && count=$((count + 1))
	done
	echo $count
}

launch() {
	local branch=$1 tip=$2; shift 2
	local name folder tmux_name command
	[[ "$tip" =~ ^[0-9a-f]{40}$ ]] || fail "the tip must be a full 40-hex commit id (git rev-parse output), not '$tip'"
	git -C "$MAIN_CHECKOUT" fetch -q origin "$branch" || fail "branch $branch is not on origin"
	git -C "$MAIN_CHECKOUT" merge-base --is-ancestor "$tip" "origin/$branch" || fail "tip $tip is not on origin/$branch: push first"
	name=$(run_name "$branch" "$tip")
	folder=$MAIN_CHECKOUT/data/cloud/$name
	while [ "$(pending_runs)" -ge $MAX_PARALLEL_RUNS ]; do
		echo "cloud-test: $MAX_PARALLEL_RUNS cloud runs pending, waiting for one to finish" >&2; sleep $POLL_SECONDS
	done
	mkdir -p "$folder"
	cloud_prompt "$(printf '%q ' ${on_main[@]+"${on_main[@]}"} "$branch" "$tip" "$@")" > "$folder/prompt.txt"
	command=(claude)
	[ -n "${CLOUD_TEST_MODEL:-}" ] && command+=(--model "$CLOUD_TEST_MODEL")
	[ "${CLOUD_TEST_FAST:-0}" = 1 ] && command+=(--settings '{"fastMode": true}')
	command+=(--cloud "$(cat "$folder/prompt.txt")")
	tmux_name=cloud-test-$(printf %s "$name" | tr -c 'A-Za-z0-9-' '-' | cut -c1-40)
	tmux new-session -d -s "$tmux_name" -c "$MAIN_CHECKOUT" -x 250 \
		"unset CLAUDECODE; $(printf '%q ' "${command[@]}"); sleep 600"
	local waited=0 url=""
	while [ -z "$url" ] && [ $waited -lt $URL_WAIT_SECONDS ]; do
		sleep 3; waited=$((waited + 3))
		tmux capture-pane -p -J -t "$tmux_name" > "$folder/pane.txt" 2>/dev/null
		url=$(session_url "$folder/pane.txt")
	done
	{
		echo "branch: $branch"
		echo "tip: $tip"
		echo "tests: ${*:-all}"
		echo "session: ${url:-unknown (tmux $tmux_name)}"
		echo "fast: ${CLOUD_TEST_FAST:-0}, model: ${CLOUD_TEST_MODEL:-environment default}"
		echo "launched: $(date -u +%FT%TZ)"
		echo "credit used before: \$$(credit_used)"
	} > "$folder/launch.txt"
	[ -n "$url" ] || fail "no session URL after ${URL_WAIT_SECONDS} s; tmux attach -t $tmux_name"
	echo "cloud-test: $name running in $url"
}

collect() {
	local branch=$1 tip=$2 card=$3
	local name folder ref deadline
	name=$(run_name "$branch" "$tip")
	folder=$MAIN_CHECKOUT/data/cloud/$name
	ref=refs/heads/$RESULTS_PREFIX/$name
	deadline=$((SECONDS + WAIT_HOURS * 3600))
	until git -C "$MAIN_CHECKOUT" ls-remote --exit-code origin "$ref" > /dev/null; do
		[ $SECONDS -lt $deadline ] || fail "no result on $RESULTS_PREFIX/$name after $WAIT_HOURS h ($(grep session: "$folder/launch.txt" 2>/dev/null))"
		sleep $POLL_SECONDS
	done
	mkdir -p "$folder"
	git -C "$MAIN_CHECKOUT" fetch -q origin "+$ref:refs/remotes/origin/$RESULTS_PREFIX/$name" || fail "fetch of $ref failed"
	git -C "$MAIN_CHECKOUT" show "origin/$RESULTS_PREFIX/$name:result.md" > "$folder/result.md"
	git -C "$MAIN_CHECKOUT" show "origin/$RESULTS_PREFIX/$name:log.txt.gz" > "$folder/log.txt.gz"
	echo "credit used after: \$$(credit_used) (shared by all cloud sessions running meanwhile)" >> "$folder/result.md"
	grep -h '^session:\|^credit used before' "$folder/launch.txt" >> "$folder/result.md" 2>/dev/null
	git -C "$MAIN_CHECKOUT" push -q origin --delete "$RESULTS_PREFIX/$name" 2>/dev/null
	git -C "$MAIN_CHECKOUT" update-ref -d "refs/remotes/origin/$RESULTS_PREFIX/$name"
	cat "$folder/result.md"
	echo "(log: $folder/log.txt.gz)"
	[ -n "$card" ] && todo note "$card" "cloud test $(sed -n 's/^verdict: //p' "$folder/result.md"): $(sed -n 's/^tests: //p' "$folder/result.md") at ${tip:0:8}, $(sed -n 's/^passed: //p' "$folder/result.md"); $(sed -n 's/^wall: //p' "$folder/result.md")$(sed -n '/^failures:/,/^[a-z]/{/^  /p;}' "$folder/result.md" | head -10 | tr '\n' ';')" > /dev/null
	grep -q '^verdict: green' "$folder/result.md"
}

# ---------------------------------------------------------------- arguments

case "${1:-}" in
	--run-here) shift; run_here "$@"; exit ;;
	--run-detached) shift; run_tests_here "$@"; touch "$HERE_WORK/done"; exit ;;
	--summarize) summarize "$2" "${3:-$ENVIRONMENTAL_LIST}"; exit ;;
esac
card="" wait=1 result_only="" on_main=()
while [[ "${1:-}" == --* ]]; do
	case "$1" in
		--card) card=$2; shift 2 ;;
		--no-wait) wait=""; shift ;;
		--result) result_only=1; shift ;;
		--on-main) on_main=(--on-main); shift ;;
		*) break ;;
	esac
done
[ $# -ge 2 ] || { sed -n '2,16p' "$0"; exit 2; }
if [ -z "$result_only" ]; then launch "$@" || exit; fi
[ -n "$wait$result_only" ] && collect "$1" "$2" "$card"
