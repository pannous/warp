#!/bin/bash
# The cloud test lane (card cloud-test-lane, notes/cloud_development.md "Cloud test lane"): a branch's tests run in a
# `claude --cloud` session, a Linux VM of its own paid by the Cloud session credit, instead of the Mac's tests/queue.sh.
#   scripts/cloud-test.sh [--card <card>] [--no-wait] [--on-main|--on <base>] <branch> <tip> [filters…|--full]
#       filters as for tests/queue.sh (`-- x y` or plain `x y`; `--test tests -- x` passes target flags to cargo);
#       --full: the whole suite as test.sh runs it (--all-features, warp-lang at opt-level 2)
#       waits for the result (run it in the background), prints it, keeps it in data/cloud/<run>/ and, with --card,
#       adds it to the card (`todo note`); --no-wait only launches: fetch later with --result;
#       --on-main tests the tip merged into the current origin/main, as the Integrator's gate will (--on <base>: into
#       origin/<base>, e.g. a batch branch)
#   scripts/cloud-test.sh --result [--card <card>] <branch> <tip>     wait for and collect a launched run's result
#   scripts/cloud-test.sh --summarize <cargo test log> [environmental list]    the result's summary of any log
# The cloud session runs this same script from origin/main (the tip may predate it) with --job <job>, which reads the
# job's arguments from the branch cloud-jobs/<job> and runs them as --run-here: it checks out
# the tip, builds, tests with --no-fail-fast and pushes result.md + log.txt.gz to the branch cloud-results/<run>.
# Sessions are pooled (data/cloud/pool/): a finished session gets the next job as a follow-up message, so its VM keeps
# the checkout and cargo's build output and the next build is incremental instead of cold.
# Failures of tests that need the Mac (scripts/cloud-test-environmental.txt) are listed apart and don't make it red.
# CLOUD_TEST_FAST=1: fast mode (Opus with faster output, burns the credit ~2x faster; off by default).
# CLOUD_TEST_MODEL=<model>: the session's model (default haiku: it only repeats one command); a reused session gets it as
# `/model` before each job (`--model` on a follow-up message doesn't switch it). Fast mode applies to new sessions only.
set -uo pipefail
MODEL=${CLOUD_TEST_MODEL:-haiku}
THIS_SCRIPT=$0                 # sent along with each job

REPO=pannous/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
RESULTS_PREFIX=cloud-results
JOBS_PREFIX=cloud-jobs         # cloud-jobs/<job>: the job's arguments, so the session's prompt stays one short line
ENVIRONMENTAL_LIST=scripts/cloud-test-environmental.txt
CARGO_FLAGS_WITH_VALUE='--test|--features|-F|-p|--package|--bin|--example|--bench|-j|--jobs|--target'   # `--test tests`: the next word is the flag's, not a filter
MAX_PARALLEL_RUNS=5           # cloud sessions running a job at once (they share the account's limits, notes/cloud_development.md)
PENDING_MINUTES=90             # a job older than this without a result: its session is dropped from the pool (a full run takes ~25)
POOL=/Users/me/dev/angles/warp/data/cloud/pool   # one file per cloud session: its current job, or idle
MAX_CLOUD_SESSIONS=8           # pool files plus starting ones: every session stays in the user's session list
JOBS_PER_SESSION=30            # then the session's context is cleared (/clear): each job adds a little to it
SESSION_NAME="Cloud Test Lane" # display name of new sessions; the pool addresses them by session_id
DROPPED_SESSIONS=/Users/me/dev/angles/warp/data/cloud/dropped-sessions.txt   # to archive in the web UI
POLL_SECONDS=60                # GitHub API quota is shared by every session: ls-remote once a minute
WAIT_HOURS=4
URL_WAIT_SECONDS=90
TEST_TIME_LIMIT_SECONDS=2400  # the test run is killed after this (a hanging test), its unfinished tests named; the full suite takes ~900 s
STALL_SECONDS=900              # …or once no test has finished for this long (the slowest, the self-parser, takes ~230 s alone)
WATCH_SECONDS=30               # how often the stall watchdog counts finished tests
HERE_WAIT_SECONDS=570          # one --run-here call blocks at most this long, under the cloud Bash tool's 10 min
HERE_ROOT=${TMPDIR:-/tmp}/cloud-test # in the VM: tree/ (the checkout, kept for the next job), jobs/<job>/
FULL_FLAGS=(--all-features --config "profile.test.package.warp-lang.opt-level=2")
USAGE_SCRIPT=$HOME/dev/bin/claude-usage.sh
RUST_TARGETS="wasm32-wasip1 wasm32-wasip2 wasm32-unknown-unknown" # package and crate-wrapper builds in the tests

fail() { echo "cloud-test: $*" >&2; exit 1; }
run_name() { echo "$(printf %s "$1" | tr -c 'A-Za-z0-9._-' '-')-${2:0:8}"; }
credit_used() { "$USAGE_SCRIPT" 2>/dev/null | sed -n 's/^iguana_necktie: \$\([0-9.]*\) .*/\1/p'; }

# ---------------------------------------------------------------- the cloud side

# A run that can't get to testing (tip missing, merge conflict) still pushes a red result, so nobody waits for it
abort() {
	printf 'run: %s\nbranch: %s\ntip: %s\nverdict: red\nerror: %s\n' "$name" "$branch" "$tip" "$*" > "$HERE_WORK/result.md"
	echo "$*" >> "$HERE_WORK/log.txt"
	gzip -kf "$HERE_WORK/log.txt"
	cd "$CLOUD_REPO" && push_result "$name"
	exit 1
}

# The run itself, detached from the session's Bash tool: checkout, build, test, summarize, push
run_tests_here() {
	CLOUD_REPO=$PWD
	local base=""
	[ "$1" = "--on" ] && base=$2 && shift 2
	local branch=$1 tip=$2; shift 2
	local name started build_seconds test_seconds checkout lock_matches=""
	name=$(run_name "$branch" "$tip")
	started=$SECONDS
	local branch_ref
	for branch_ref in "$branch" main vendor $base; do
		git fetch -q origin "+refs/heads/$branch_ref:refs/remotes/origin/$branch_ref" 2>&1 | tail -3
	done
	git cat-file -e "$tip^{commit}" 2>/dev/null || git fetch -q origin "$tip" || abort "tip $tip not found on origin"
	checkout=$HERE_ROOT/tree
	local warm=no
	if [ -e "$checkout/.git" ]; then
		warm=yes
		git -C "$checkout" merge --abort 2>/dev/null
		{ git -C "$checkout" reset -q --hard && git -C "$checkout" checkout -q -f --detach "$tip" && git -C "$checkout" clean -fdq; } ||
			abort "checkout of $tip failed"
	else
		git worktree prune && git worktree add -q --detach "$checkout" "$tip" || abort "checkout of $tip failed"
	fi
	git show "origin/main:$ENVIRONMENTAL_LIST" > "$HERE_WORK/environmental.txt" 2>/dev/null
	cd "$checkout" || abort "no checkout"
	local tested=$tip
	if [ -n "$base" ]; then
		git -c user.name=pannous -c user.email=info@pannous.com merge -q --no-edit "origin/$base" || abort "$tip does not merge cleanly into origin/$base"
		tested="$tip merged into origin/$base $(git rev-parse "origin/$base")"
	fi
	rustup target add $RUST_TARGETS >/dev/null 2>&1
	local source_flags=(--config net.offline=false)
	if git diff --quiet origin/vendor HEAD -- Cargo.lock 2>/dev/null; then
		lock_matches=1
		git archive origin/vendor vendor | tar -x
		source_flags=(--offline --config 'source.crates-io.replace-with="vendored-sources"' --config 'source.vendored-sources.directory="vendor"')
	fi
	local test_args=() filters=() past_separator="" flag_value_next=""
	if [ "${1:-}" = "--full" ]; then
		test_args=("${FULL_FLAGS[@]}")
	else
		for argument in "$@"; do
			if [ "$argument" = "--" ]; then past_separator=1
			elif [ -z "$past_separator" ] && [ -n "$flag_value_next" ]; then test_args+=("$argument"); flag_value_next=""
			elif [ -n "$past_separator" ] || [[ "$argument" != -* ]]; then filters+=("$argument")
			else
				test_args+=("$argument")
				[[ "$argument" =~ ^($CARGO_FLAGS_WITH_VALUE)$ ]] && flag_value_next=1
			fi
		done
	fi
	export WARP_NO_WINDOW=1 WARP_CRASH_CARDS=0 CARGO_TERM_COLOR=never
	local build_start=$SECONDS
	cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast --no-run > "$HERE_WORK/log.txt" 2>&1
	local build_status=$?
	build_seconds=$((SECONDS - build_start))
	local test_start=$SECONDS
	[ $build_status -eq 0 ] && run_watched "$HERE_WORK/log.txt" \
		cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast -- "${filters[@]}"
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
		echo "warm VM: $warm (the checkout and build output of an earlier job)"
		echo "crates: $([ -n "$lock_matches" ] && echo "vendor branch (offline)" || echo "crates.io (vendor branch's Cargo.lock differs)")"
		echo "finished: $(date -u +%FT%TZ)"
	} > "$HERE_WORK/result.md"
	gzip -kf "$HERE_WORK/log.txt"
	push_result "$name"
}

# The command, its output appended to the log, killed with all its processes after TEST_TIME_LIMIT_SECONDS or once no
# test finished for STALL_SECONDS (a hang held batch-3's session 85 min); the summary then names the unfinished tests
run_watched() {
	local log=$1 runner finished last_finished=-1 still=0; shift
	timeout --kill-after=30 $TEST_TIME_LIMIT_SECONDS "$@" >> "$log" 2>&1 &   # its own process group: a kill reaches the test binaries
	runner=$!
	while kill -0 $runner 2>/dev/null; do
		sleep $WATCH_SECONDS
		finished=$(grep -c '^test .* \.\.\. ' "$log")
		if [ "$finished" = "$last_finished" ]; then still=$((still + WATCH_SECONDS)); else still=0 last_finished=$finished; fi
		if [ $still -ge $STALL_SECONDS ]; then
			echo "cloud-test: KILLED: no test finished for $STALL_SECONDS s" >> "$log"
			kill -TERM $runner
			break
		fi
	done
	wait $runner
	[ $? -eq 124 ] && echo "cloud-test: KILLED after the time limit of $TEST_TIME_LIMIT_SECONDS s" >> "$log"
}

# Counts, failing tests with the first line of their panic, environmental ones apart; verdict green|red
summarize() {
	local log=$1 environmental=$2
	awk -v environmental="$environmental" '
		BEGIN { while ((getline line < environmental) > 0) { sub(/[ \t]*#.*/, "", line); if (line != "") patterns[++count] = line } }
		/^test result:/ { for (i = 1; i <= NF; i++) { if ($(i) ~ /^(passed|failed|ignored)/) { key = $(i); sub(/[;,]/, "", key); total[key] += $(i - 1) } } finished++ }
		/^ +(Running|Doc-tests) / { started++ }
		/^test [^ ]+ \.\.\. .*FAILED/ { failed[$2] = 1; order[++failures] = $2 }
		/^test [^ ]+ has been running for over/ { slow[$2] = 1 }
		/^test [^ ]+ \.\.\. / { finished_test[$2] = 1 }
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
			if (incomplete) printf "crashed or killed test binaries (no result line): %d\n", incomplete
			for (name in slow) if (!(name in finished_test)) printf "  never finished: %s\n", name
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
	if [ -n "$commit" ] && git push -q -f origin "$commit:refs/heads/$RESULTS_PREFIX/$name" > "$HERE_WORK/push.log" 2>&1; then
		touch "$HERE_WORK/pushed"
	fi
}

# What the cloud session calls, again and again until DONE: starts the run once, then waits for it a while. It prints
# one line only, every token the session reads is paid from the credit: the result itself reaches the Mac through git
run_here() {
	HERE_WORK=$HERE_ROOT/jobs/$1; shift
	mkdir -p "$HERE_WORK"
	if [ ! -e "$HERE_WORK/started" ]; then
		touch "$HERE_WORK/started"
		nohup bash "$0" --run-detached "${HERE_WORK##*/}" "$@" > "$HERE_WORK/runner.log" 2>&1 &
	fi
	local waited=0
	while [ ! -e "$HERE_WORK/done" ] && [ $waited -lt $HERE_WAIT_SECONDS ]; do sleep 15; waited=$((waited + 15)); done
	if [ ! -e "$HERE_WORK/done" ]; then
		echo "CLOUD TEST RUNNING ($(grep -c '^test .* \.\.\. ' "$HERE_WORK/log.txt" 2>/dev/null) tests so far): call the same command again"
	elif [ -e "$HERE_WORK/pushed" ]; then
		echo "CLOUD TEST DONE: $(sed -n 's/^verdict: //p' "$HERE_WORK/result.md"), $(sed -n 's/^passed: //p; s/^error: //p' "$HERE_WORK/result.md")"
	else
		echo "CLOUD TEST DONE, but the result was not pushed: $(tail -2 "$HERE_WORK/push.log" "$HERE_WORK/runner.log" 2>/dev/null | tr '\n' ' ' | cut -c1-300)"
	fi
}

# --run-here with the arguments of cloud-jobs/<job>, fetched once per job
run_job() {
	local job=$1 arguments=$HERE_ROOT/jobs/$1/arguments
	if [ ! -s "$arguments" ]; then
		mkdir -p "${arguments%/*}"
		git fetch -q origin "+refs/heads/$JOBS_PREFIX/$job:refs/$JOBS_PREFIX/$job" && git show "refs/$JOBS_PREFIX/$job:arguments" > "$arguments" ||
			{ echo "CLOUD TEST DONE: red, no job $JOBS_PREFIX/$job on origin"; return 1; }
	fi
	local job_arguments=()
	mapfile -t job_arguments < "$arguments"
	run_here "$job" "${job_arguments[@]}"
}

# ---------------------------------------------------------------- the Mac side

cloud_prompt() {
	cat <<-EOF
	Run this in the foreground (timeout 600000), again while it prints CLOUD TEST RUNNING, then reply with only its last line. Nothing else.
	git fetch -q origin $JOBS_PREFIX/$1 && git show FETCH_HEAD:cloud-test.sh > /tmp/cloud-test.sh && bash /tmp/cloud-test.sh --job $1
	EOF
}

# The job's arguments, one per line, and this script as the one commit of cloud-jobs/<job>: the cloud runs the very
# version that launched the job, never a newer or older origin/main
push_job() {
	local job=$1 arguments script tree commit output; shift
	arguments=$(printf '%s\n' "$@" | git -C "$MAIN_CHECKOUT" hash-object -w --stdin)
	script=$(git -C "$MAIN_CHECKOUT" hash-object -w --stdin < "$THIS_SCRIPT")
	tree=$(printf '100644 blob %s\targuments\n100644 blob %s\tcloud-test.sh\n' "$arguments" "$script" | git -C "$MAIN_CHECKOUT" mktree)
	commit=$(git -C "$MAIN_CHECKOUT" commit-tree "$tree" -m "cloud test job $job")
	output=$(git -C "$MAIN_CHECKOUT" push -q origin "$commit:refs/heads/$JOBS_PREFIX/$job" 2>&1) ||
		fail "push of $JOBS_PREFIX/$job failed: $output"   # quiet otherwise: GitHub offers a pull request for every new branch
}

session_url() { grep -o 'https://claude.ai/code/session_[A-Za-z0-9_]*' "$1" | head -1; }

lock_pool() {
	mkdir -p "$POOL"
	until mkdir "$POOL.lock" 2>/dev/null; do
		[ -n "$(find "$POOL.lock" -maxdepth 0 -mmin +2 2>/dev/null)" ] && rmdir "$POOL.lock" 2>/dev/null
		sleep 1
	done
}
unlock_pool() { rmdir "$POOL.lock" 2>/dev/null; }
pool_field() { sed -n "s/^$2: //p" "$1"; }
mark_session() { printf 'job: %s\nsince: %s\njobs: %s\n' "$2" "$(date +%s)" "$3" > "$1"; }

# A pooled session is free once its job's result is in (collected, or still on its results branch); a job that
# never answered drops its session, which may be gone
session_is_free() {
	local file=$1 job since run
	job=$(pool_field "$file" job); since=$(pool_field "$file" since)
	[ "$job" = idle ] && return 0
	run=${job%-*}
	git -C "$MAIN_CHECKOUT" ls-remote --exit-code origin "refs/heads/$RESULTS_PREFIX/$run" > /dev/null && return 0
	[ $(($(date +%s) - since)) -gt $((PENDING_MINUTES * 60)) ] && drop_session "$file" "its job $job never answered"
	return 1
}

# Loud: the session stays in the user's session list until it is archived there
drop_session() {
	local file=$1 reason=$2 url=https://claude.ai/code/${1##*/}
	rm -f "$file"
	echo "$(date -u +%FT%TZ) $url $reason" >> "$DROPPED_SESSIONS"
	echo "cloud-test: dropped $url from the pool ($reason): archive it in the web UI ($DROPPED_SESSIONS)" >&2
}

# Under the pool lock: the session already running a job of this run (a second launch of the same branch and tip)
pending_session_of() {
	local name=$1 file job
	for file in "$POOL"/session_* "$POOL"/starting-*; do
		[ -e "$file" ] || continue
		job=$(pool_field "$file" job)
		[[ "$job" == "$name"-* ]] && ! session_is_free "$file" && { echo "${file##*/}"; return; }
	done
}

# Under the pool lock: a free session for the job (its id, marked busy; jobs 1 = its context is to be cleared first),
# "new" when one may be started, else nothing
claim_session() {
	local job=$1 file jobs busy=0 sessions
	for file in "$POOL"/session_*; do
		[ -e "$file" ] || continue
		jobs=$(pool_field "$file" jobs)
		if session_is_free "$file"; then
			[ "$jobs" -ge $JOBS_PER_SESSION ] && jobs=0
			mark_session "$file" "$job" $((jobs + 1)); echo "${file##*/}"; return
		fi
		[ -e "$file" ] && busy=$((busy + 1))
	done
	busy=$((busy + $(ls "$POOL" | grep -c '^starting-')))
	sessions=$(ls "$POOL" | grep -c -E '^(session_|starting-)')
	[ $busy -lt $MAX_PARALLEL_RUNS ] && [ $sessions -lt $MAX_CLOUD_SESSIONS ] && { mark_session "$POOL/starting-$job" "$job" 1; echo new; }
}

start_session() {
	local job=$1 folder=$2 tmux_name command waited=0 url=""
	command=(claude)
	command+=(--model "$MODEL" --name "$SESSION_NAME")
	[ "${CLOUD_TEST_FAST:-0}" = 1 ] && command+=(--settings '{"fastMode": true}')
	command+=(--cloud "$(cat "$folder/prompt.txt")")
	tmux_name=cloud-test-$(printf %s "$job" | tr -c 'A-Za-z0-9-' '-' | cut -c1-40)
	tmux new-session -d -s "$tmux_name" -c "$MAIN_CHECKOUT" -x 250 "unset CLAUDECODE; $(printf '%q ' "${command[@]}"); sleep 600"
	while [ -z "$url" ] && [ $waited -lt $URL_WAIT_SECONDS ]; do
		sleep 3; waited=$((waited + 3))
		tmux capture-pane -p -J -t "$tmux_name" > "$folder/pane.txt" 2>/dev/null
		url=$(session_url "$folder/pane.txt")
	done
	[ -n "$url" ] || { rm -f "$POOL/starting-$job"; fail "no session URL after ${URL_WAIT_SECONDS} s; tmux attach -t $tmux_name"; }
	mv "$POOL/starting-$job" "$POOL/${url##*/}"
	echo "$url"
}

launch() {
	local branch=$1 tip=$2; shift 2
	local name folder job session url=""
	[[ "$tip" =~ ^[0-9a-f]{40}$ ]] || fail "the tip must be a full 40-hex commit id (git rev-parse output), not '$tip'"
	git -C "$MAIN_CHECKOUT" fetch -q origin "$branch" || fail "branch $branch is not on origin"
	git -C "$MAIN_CHECKOUT" merge-base --is-ancestor "$tip" "origin/$branch" || fail "tip $tip is not on origin/$branch: push first"
	name=$(run_name "$branch" "$tip")
	job=$name-$(date +%s)
	folder=$MAIN_CHECKOUT/data/cloud/$name
	mkdir -p "$folder"
	rm -f "$folder/result.md"
	lock_pool; session=$(pending_session_of "$name"); unlock_pool
	if [ -n "$session" ]; then
		echo "cloud-test: $name already runs in https://claude.ai/code/${session#starting-}: waiting for that result" >&2
		return
	fi
	push_job "$job" ${base[@]+"${base[@]}"} "$branch" "$tip" "$@"
	cloud_prompt "$job" > "$folder/prompt.txt"
	while [ -z "$url" ]; do
		lock_pool; session=$(claim_session "$job"); unlock_pool
		case "$session" in
			new) url=$(start_session "$job" "$folder") || exit ;;
			session_*)
				[ "$(pool_field "$POOL/$session" jobs)" = 1 ] && claude -p /clear --cloud "$session" > "$folder/clear.txt" 2>&1
				claude -p "/model $MODEL" --cloud "$session" > "$folder/model.txt" 2>&1
				if claude -p "$(cat "$folder/prompt.txt")" --cloud "$session" > "$folder/send.txt" 2>&1; then
					url=https://claude.ai/code/$session
				else
					lock_pool; drop_session "$POOL/$session" "refused the job: $(tail -1 "$folder/send.txt")"; unlock_pool
				fi ;;
			*) echo "cloud-test: all pooled cloud sessions busy (at most $MAX_PARALLEL_RUNS runs, $MAX_CLOUD_SESSIONS sessions), waiting for one" >&2; sleep $POLL_SECONDS ;;
		esac
	done
	{
		echo "branch: $branch"
		echo "tip: $tip"
		echo "tests: ${*:-all}"
		echo "job: $job"
		echo "session: $url"
		echo "fast: ${CLOUD_TEST_FAST:-0}, model: $MODEL"
		echo "launched: $(date -u +%FT%TZ)"
		echo "credit used before: \$$(credit_used)"
	} > "$folder/launch.txt"
	echo "cloud-test: $name running in $url"
}

# The job's session takes the next job
release_session() {
	local job=$1 file
	lock_pool
	for file in "$POOL"/session_*; do
		[ "$(pool_field "$file" job 2>/dev/null)" = "$job" ] && mark_session "$file" idle "$(pool_field "$file" jobs)"
	done
	unlock_pool
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
	git -C "$MAIN_CHECKOUT" push -q origin --delete "$RESULTS_PREFIX/$name" "$JOBS_PREFIX/$(pool_field "$folder/launch.txt" job)" 2>/dev/null
	git -C "$MAIN_CHECKOUT" update-ref -d "refs/remotes/origin/$RESULTS_PREFIX/$name"
	release_session "$(pool_field "$folder/launch.txt" job)"
	cat "$folder/result.md"
	echo "(log: $folder/log.txt.gz)"
	[ -n "$card" ] && todo note "$card" "cloud test $(sed -n 's/^verdict: //p' "$folder/result.md"): $(sed -n 's/^tests: //p' "$folder/result.md") at ${tip:0:8}, $(sed -n 's/^passed: //p' "$folder/result.md"); $(sed -n 's/^wall: //p' "$folder/result.md")$(sed -n '/^failures:/,/^[a-z]/{/^  /p;}' "$folder/result.md" | head -10 | tr '\n' ';')" > /dev/null
	grep -q '^verdict: green' "$folder/result.md"
}

# ---------------------------------------------------------------- arguments

# one function, parsed whole before it runs: editing this file during a long wait then can't break the waiting run
main() {
	case "${1:-}" in
		--job) run_job "$2"; exit ;;
		--run-here) shift; run_here "$@"; exit ;;
		--run-detached) HERE_WORK=$HERE_ROOT/jobs/$2; shift 2; mkdir -p "$HERE_WORK"; trap 'touch "$HERE_WORK/done"' EXIT; run_tests_here "$@"; exit ;;
		--summarize) summarize "$2" "${3:-$ENVIRONMENTAL_LIST}"; exit ;;
	esac
	card="" wait=1 result_only="" base=()
	while [[ "${1:-}" == --* ]]; do
		case "$1" in
			--card) card=$2; shift 2 ;;
			--no-wait) wait=""; shift ;;
			--result) result_only=1; shift ;;
			--on-main) base=(--on main); shift ;;
			--on) base=(--on "$2"); shift 2 ;;
			*) break ;;
		esac
	done
	[ $# -ge 2 ] || { sed -n '2,16p' "$0"; exit 2; }
	if [ -z "$result_only" ]; then launch "$@" || exit; fi
	[ -n "$wait$result_only" ] && collect "$1" "$2" "$card"
}
main "$@"; exit
