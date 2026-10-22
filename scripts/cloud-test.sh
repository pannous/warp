#!/bin/bash
# The cloud test lane (card cloud-test-lane, notes/cloud_development.md "Cloud test lane"): a branch's tests run in a
# `claude --cloud` session, a Linux VM of its own paid by the Cloud session credit, instead of the Mac's tests/queue.sh.
#   scripts/cloud-test.sh [--card <card>] [--no-wait] [--on-main|--on <base>] [--env VAR=value…] [--optimized] [--pure-hello] [--gate] <branch> <tip> [filters…|--full]
#       --env: a setting of the test run alone (not the build: every setting of a tip shares one build)
#       --optimized: warp-lang at opt-level 2, as the full suite and the gate build it: the warp self tests need it (in
#       a debug build the self build takes ~1100 s and `warp self` runs out of its 60 s; card cloud-lane-narrow)
#       --pure-hello: run the ignored end-to-end `warp pure hello` check at test opt-level 2
#       --gate: the Integrator's run (implied for batch-*/express* branches): it may use every run slot and goes first
#       filters as for tests/queue.sh (`-- x y` or plain `x y`; `--test tests -- x` passes target flags to cargo);
#       --full: the whole suite as test.sh runs it (--all-features, warp-lang at opt-level 2), sharded: FULL_SHARDS
#       jobs on as many VMs, each every FULL_SHARDS-th test file, merged into one result
#       waits for the result (run it in the background), prints it, keeps it in data/cloud/<run>/ and, with --card,
#       adds it to the card (`todo note`); --no-wait only launches: fetch later with --result;
#       --on-main tests the tip merged into the current origin/main, as the Integrator's gate will (--on <base>: into
#       origin/<base>, e.g. a batch branch)
#   scripts/cloud-test.sh --result [same options] [--card <card>] <branch> <tip> [same filters]   collect a --no-wait run
#   scripts/cloud-test.sh --summarize <cargo test log> [environmental list] [known failures]    the result's summary of any log
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
SLOW_FILES=""                  # sent along with each --full shard's job (slow_files)

REPO=pannous/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
RESULTS_PREFIX=cloud-results
JOBS_PREFIX=cloud-jobs         # cloud-jobs/<job>: the job's arguments, so the session's prompt stays one short line
ENVIRONMENTAL_LIST=scripts/cloud-test-environmental.txt
NO_TESTS_RAN="NO TESTS RAN: "   # the summary line of a run that executed no test: red (a filter fault, its session stays pooled)
SHOWN_ITEMS=12                 # lines of a real failure's panic message shown under it (a slice test's failing samples)
KNOWN_FAILURES=notes/known_failures.md   # read in the tested checkout, as the gate reads it at the tested tip
CARGO_FLAGS_WITH_VALUE='--config|--test|--features|-F|-p|--package|--bin|--example|--bench|-j|--jobs|--target'   # `--test tests`: the next word is the flag's, not a filter
MAX_PARALLEL_RUNS=7           # cloud sessions running a job at once (they share the account's limits, notes/cloud_development.md)
WORKER_RUNS=$((MAX_PARALLEL_RUNS - 2))   # slots a worker's runs may hold: the rest is kept for gate runs (user 2026-10-21)
GATE_BRANCHES='^(batch-|express)'        # branches whose runs are gate runs without --gate
PENDING_MINUTES=90             # a job older than this without a result: its session is dropped from the pool (a full run's shard takes ~15)
POOL=/Users/me/dev/angles/warp/data/cloud/pool   # one file per cloud session: its current job, or idle
MAX_CLOUD_SESSIONS=10          # pool files plus starting ones: every session stays in the user's session list
JOBS_PER_SESSION=30            # then the session's context is cleared (/clear): each job adds a little to it
SESSION_NAME="Cloud Test Lane" # display name of new sessions; the pool addresses them by session_id
DROPPED_SESSIONS=/Users/me/dev/angles/warp/data/cloud/dropped-sessions.txt   # to archive in the web UI
POLL_SECONDS=60                # GitHub API quota is shared by every session: ls-remote once a minute
WAIT_HOURS=4
URL_WAIT_SECONDS=90
MIN_FREE_GB=15                 # less free disk before a build: the warm VM's build output is pruned (a build ran out of disk)
TEST_TIME_LIMIT_SECONDS=3600   # the test run is killed after this (a hanging test), its unfinished tests named; a full run on a 4-core VM takes 1600-2400 s since the warp self tests rebuild their compiler (batch-1, batch-2 killed at 2400)
STALL_SECONDS=900              # …or once no test has finished for this long while the tests use no CPU (a hang waiting)
BUSY_STALL_SECONDS=1800        # …or for this long while they compute: the self build behind the warp self tests runs up to 900 s (its WARP_TIME_LIMIT) with no test finishing (card cloud-lane-narrow)
BUSY_CPU_SECONDS=5             # CPU seconds the tests use per watch that count as computing
WATCH_SECONDS=30               # how often the stall watchdog counts finished tests
PURE_HELLO_TEST=parser::test_warp_pure::warp_pure_runs_hello_world
OPTIMIZED_TEST_PROFILE=(--config profile.test.package.warp-lang.opt-level=2)
PURE_HELLO_ARGS=("${OPTIMIZED_TEST_PROFILE[@]}" -- "$PURE_HELLO_TEST" --ignored --exact)
HERE_WAIT_SECONDS=570          # one --run-here call blocks at most this long, under the cloud Bash tool's 10 min
HERE_ROOT=${TMPDIR:-/tmp}/cloud-test # in the VM: tree/ (the checkout, kept for the next job), jobs/<job>/
FULL_FLAGS=(--all-features "${OPTIMIZED_TEST_PROFILE[@]}")
FULL_SHARDS=5                  # a --full run's jobs: one VM ran the suite in 1300-3600 s, killed at the limit (card cloud-lane-full)
SHARD_FLAG=--shard             # --full --shard <i>/<n>: one shard's job, on the cloud side
SHARD_HEADER="==== shard "     # a merged log's line before each shard's log
RERUN_MARK="cloud-test: RERUN" # a killed run's failed tests run once more for their panic texts: lines after it count no test
RERUN_SECONDS=600
LAST_FINISHES_SHOWN=6          # result.md names the tests that finished last, with their time into the test run
USAGE_SCRIPT=$HOME/dev/bin/claude-usage.sh
RUST_TARGETS="wasm32-wasip1 wasm32-wasip2 wasm32-unknown-unknown" # package and crate-wrapper builds in the tests

fail() { echo "cloud-test: $*" >&2; exit 1; }
run_name() {
	local branch=$1 tip=$2 signature
	shift 2
	signature=$(printf '%s\0' "$@" | git hash-object --stdin | cut -c1-12)
	echo "$(printf %s "$branch" | tr -c 'A-Za-z0-9._-' '-')-${tip:0:8}-$signature"
}
# Test names hold no space: a filter argument with spaces ("a b c", a quoted list) is several filters. Passed whole, libtest
# matched it against no test (5 runs of 0 tests on 2026-10-21, their sessions dropped as faulty VMs)
split_filters() {
	local argument words
	for argument in "$@"; do
		read -ra words <<< "$argument"
		[ ${#words[@]} -gt 1 ] && echo "cloud-test: the filter '$argument' is split at its spaces into ${#words[@]} filters" >&2
		[ ${#words[@]} -gt 0 ] && printf '%s\n' "${words[@]}"
	done
}
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

free_gb() { df -Pk . | awk 'NR == 2 { print int($4 / 1048576) }'; }

# A warm VM's target/ grows with every job's branch, features and profile until a build runs out of disk: below
# MIN_FREE_GB the incremental caches go, then if need be all of target/ (a cold build). Prints what it found and did
make_room() {
	local free
	free=$(free_gb)
	[ "$free" -ge $MIN_FREE_GB ] && { echo "$free of $(df -Pk . | awk 'NR == 2 { print int($2 / 1048576) }') GB free"; return; }
	rm -rf target/*/incremental target/*/*/incremental
	[ "$(free_gb)" -ge $MIN_FREE_GB ] && { echo "$free GB free, $(free_gb) GB after dropping the incremental caches"; return; }
	rm -rf target
	echo "$free GB free, $(free_gb) GB after dropping target/"
}

# compiler.wasm and compiler-self.wasm (warp pure / warp self, src/compiler/standalone.rs) built by an earlier job from
# another lib/ or warp build: the tests ran on them with only a notice, so a verdict could be the earlier branch's
# (card cloud-stale-compiler: "unknown name Ascii" from a compiler built before byte_codes.warp). Kept while lib/ and
# the Rust sources are those they were built from (compiler.wasm takes ~540 s); prints what it did
drop_stale_compilers() {
	local stamp=$HERE_ROOT/compiler-sources sources dropped
	sources=$(git rev-parse HEAD:lib HEAD:src HEAD:crates HEAD:build.rs HEAD:Cargo.lock 2>/dev/null | tr '\n' ' ')
	[ "$sources" = "$(cat "$stamp" 2>/dev/null)" ] && return
	dropped=$(find target data \( -name compiler.wasm -o -name compiler-self.wasm \) -print -delete 2>/dev/null | wc -l)
	echo "$sources" > "$stamp"
	echo "cloud-test: lib/ or the Rust sources differ from the last job's: dropped $((dropped)) compiler build files"
}

detect_build_environment() {
	local status=$1 log=$2
	[ "$status" -ne 0 ] && grep -q 'No space left on device' "$log" && echo "disk exhausted after a clean rebuild"
}

# The filters among $@ that no built test's name contains (cargo test's substring match); run_tests_here's flags
unmatched_filters() {
	local listed filter
	listed=$(cargo test "${source_flags[@]}" "${test_args[@]}" -- --list 2>/dev/null | sed -n 's/: test$//p')
	for filter in "$@"; do
		[[ "$filter" == -* ]] || grep -qF -- "$filter" <<< "$listed" || printf '%s ' "$filter"
	done
}

# The test file of each test name on stdin: <folder>::<file>::
test_files() { awk -F'::' '{ print (NF < 3 ? $0 : $1 "::" $2 "::") }' | sort -u; }

# The filters of shard <i>/<n>: every n-th test file of the built tests, the slow ones (the job's file `slow`) dealt
# first so that they spread over the shards (all four self-hosted files once fell to one shard: 1351 s of tests, the
# others 300 s), then the rest in sorted order
shard_filters() {
	local files
	[ -f "$HERE_WORK/slow" ] || : > "$HERE_WORK/slow"   # missing, grep -v printed no file at all: no filter runs every test
	files=$(cargo test "${source_flags[@]}" "${test_args[@]}" -- --list 2>/dev/null | sed -n 's/: test$//p' | test_files)
	{ grep -Fxf "$HERE_WORK/slow" <<< "$files"; grep -vFxf "$HERE_WORK/slow" <<< "$files"; } |
		awk -v shard="${1%/*}" -v shards="${1#*/}" '(NR - 1) % shards == shard - 1'
}

# A killed test binary prints no panic text of its failed tests (libtest prints them at its end): those run once more,
# alone; $@ is the cargo command before its `--`
rerun_killed_failures() {
	local log=$1 failed; shift
	grep -q '^cloud-test: KILLED' "$log" || return
	failed=$(awk '/^test [^ ]+ \.\.\. .*FAILED/ { failed[$2] = 1 } /^---- [^ ]+ stdout ----$/ { told[$2] = 1 }
		END { for (name in failed) if (!(name in told)) print name }' "$log")
	[ -n "$failed" ] || return
	echo "$RERUN_MARK of the killed run's failed tests, for their panic texts" >> "$log"
	timeout --kill-after=30 $RERUN_SECONDS "$@" -- --exact $failed >> "$log" 2>&1
}

write_summary() {
	local log=$1 environmental=$2 known=$3 environment=$4
	if [ -n "$environment" ]; then
		printf 'verdict: environmental\nenvironment: %s\npassed: 0, failed: 0 (build did not reach tests), ignored: 0\n' "$environment"
	else
		summarize "$log" "$environmental" "$known"
	fi
}

# The run itself, detached from the session's Bash tool: checkout, build, test, summarize, push
run_tests_here() {
	CLOUD_REPO=$PWD
	local base="" settings=()
	while true; do
		case "${1:-}" in
			--on) base=$2; shift 2 ;;
			--env) settings+=("$2"); shift 2 ;;
			*) break ;;
		esac
	done
	local branch=$1 tip=$2; shift 2
	local name started build_seconds test_seconds checkout lock_matches="" build_environment="" setting
	local identity=()
	[ -n "$base" ] && identity+=(--on "$base")
	for setting in ${settings[@]+"${settings[@]}"}; do identity+=(--env "$setting"); done
	name=$(run_name "$branch" "$tip" "${identity[@]}" "$@")
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
	# the vendored sources in a config above the checkout, not --config flags: a cargo the tests run themselves (the
	# libwarp_runtime build of tests/common runtime_library_dir) reads it too (card cloud-fxprof)
	local vendor_config=$HERE_ROOT/.cargo/config.toml
	rm -f "$vendor_config"
	if git diff --quiet origin/vendor HEAD -- Cargo.lock 2>/dev/null; then
		lock_matches=1
		git archive origin/vendor vendor | tar -x
		mkdir -p "$(dirname "$vendor_config")"
		printf '[net]\noffline = true\n[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "%s"\n' \
			"$checkout/vendor" > "$vendor_config"
		source_flags=(--offline)
	fi
	local test_args=() filters=() past_separator="" flag_value_next="" shard=""
	if [ "${1:-}" = "--full" ]; then
		test_args=("${FULL_FLAGS[@]}")
		[ "${2:-}" = "$SHARD_FLAG" ] && shard=$3
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
	# WARP_COMMIT: the build's name without its reflog (build.rs), so the next job on a commit of the same tree reuses it
	export WARP_NO_WINDOW=1 WARP_CRASH_CARDS=0 CARGO_TERM_COLOR=never WARP_COMMIT=${tip:0:9}
	local build_start=$SECONDS disk
	disk=$(make_room)
	cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast --no-run > "$HERE_WORK/log.txt" 2>&1
	local build_status=$?
	if [ $build_status -ne 0 ] && grep -q 'No space left on device' "$HERE_WORK/log.txt"; then
		rm -rf target
		disk="$disk; out of space during the build: target/ dropped, built again with $(free_gb) GB free"
		cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast --no-run > "$HERE_WORK/log.txt" 2>&1
		build_status=$?
	fi
	# a warm VM's build once held fewer tests than the tip (front-end-warnings-257e97ec: build 0 s, 6675 of 6755 tests,
	# the first filters matched nothing): a filter no built test matches forces a build anew, then is named in the log
	if [ $build_status -eq 0 ] && [ ${#filters[@]} -gt 0 ] && [ -n "$(unmatched_filters "${filters[@]}")" ]; then
		echo "cloud-test: filters matching no built test: $(unmatched_filters "${filters[@]}"): building anew" >> "$HERE_WORK/log.txt"
		find src tests crates -name '*.rs' -exec touch {} +
		cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast --no-run >> "$HERE_WORK/log.txt" 2>&1
		build_status=$?
		local unmatched
		unmatched=$(unmatched_filters "${filters[@]}")
		[ -n "$unmatched" ] && echo "$NO_TESTS_RAN filters matching no test after a fresh build: $unmatched" >> "$HERE_WORK/log.txt"
	fi
	if [ $build_status -eq 0 ] && [ -n "$shard" ]; then
		local filter
		while IFS= read -r filter; do filters+=("$filter"); done < <(shard_filters "$shard")
		echo "cloud-test: shard $shard: ${#filters[@]} test files" >> "$HERE_WORK/log.txt"
	fi
	build_environment=$(detect_build_environment "$build_status" "$HERE_WORK/log.txt")
	build_seconds=$((SECONDS - build_start))
	# the runtime stub the executable tests copy, built with the run's crate source as test.sh does: a test's own
	# `cargo build --offline` found no miniz_oxide in the VM's registry cache (card cloud-stub-offline); the page stub is
	# macOS-only
	if [ $build_status -eq 0 ]; then
		if cargo build "${source_flags[@]}" --quiet -p warp-runtime --bin warp-runtime >> "$HERE_WORK/log.txt" 2>&1; then
			export WARP_TEST_STUBS_BUILT=1
		else
			echo "cloud-test: the runtime stub's build failed, the executable tests build it themselves" >> "$HERE_WORK/log.txt"
		fi
	fi
	# the file the --env settings name in WARP_EMIT_REFUSALS (scripts/self-hosted-suite.sh): emptied before the tests,
	# its lines appended to the log after them (sent to stderr they reached the output the CLI tests check)
	local refusals=""
	for setting in ${settings[@]+"${settings[@]}"}; do
		[[ "$setting" == WARP_EMIT_REFUSALS=* ]] && refusals=${setting#*=}
	done
	[ -n "$refusals" ] && rm -f "$refusals"
	drop_stale_compilers >> "$HERE_WORK/log.txt"
	local test_start=$SECONDS
	# the settings reach the tests alone: the build stays the same for every setting, so a warm VM builds a tip once
	[ $build_status -eq 0 ] && printf 'cloud-test: in %s at %s: cargo test %s -- %s\n' "$PWD" "$(git rev-parse --short HEAD)" "${test_args[*]}" "${filters[*]}" >> "$HERE_WORK/log.txt"
	[ $build_status -eq 0 ] && run_watched "$HERE_WORK/log.txt" \
		env ${settings[@]+"${settings[@]}"} cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast -- "${filters[@]}"
	[ $build_status -eq 0 ] && rerun_killed_failures "$HERE_WORK/log.txt" \
		env ${settings[@]+"${settings[@]}"} cargo test "${source_flags[@]}" "${test_args[@]}" --no-fail-fast
	test_seconds=$((SECONDS - test_start))
	[ -n "$refusals" ] && [ -f "$refusals" ] && cat "$refusals" >> "$HERE_WORK/log.txt"
	write_summary "$HERE_WORK/log.txt" "$HERE_WORK/environmental.txt" "$KNOWN_FAILURES" "$build_environment" > "$HERE_WORK/summary.txt"
	{
		echo "run: $name"
		echo "branch: $branch"
		echo "tip: $tested"
		echo "tests: ${*:-all}"
		[ ${#settings[@]} -gt 0 ] && echo "env: ${settings[*]}"
		echo "verdict: $(sed -n 's/^verdict: //p' "$HERE_WORK/summary.txt")"
		[ $build_status -ne 0 ] && echo "build: $([ -n "$build_environment" ] && echo ENVIRONMENTAL || echo FAILED) (log.txt.gz)"
		grep -v '^verdict: ' "$HERE_WORK/summary.txt"
		echo "wall: build ${build_seconds} s, tests ${test_seconds} s, total $((SECONDS - started)) s"
		[ -s "$HERE_WORK/finish-times.txt" ] && echo "last to finish: $(sort -rn "$HERE_WORK/finish-times.txt" | head -$LAST_FINISHES_SHOWN | awk '{ printf "%s%s at %s s", NR > 1 ? ", " : "", $2, $1 }')"
		echo "machine: $(nproc 2>/dev/null) cores, $(free -g 2>/dev/null | awk '/^Mem:/ { print $2 " GB" }'), $(rustc --version)"
		echo "warm VM: $warm (the checkout and build output of an earlier job)"
		echo "disk: $disk"
		echo "crates: $([ -n "$lock_matches" ] && echo "vendor branch (offline)" || echo "crates.io (vendor branch's Cargo.lock differs)")"
		echo "finished: $(date -u +%FT%TZ)"
	} > "$HERE_WORK/result.md"
	gzip -kf "$HERE_WORK/log.txt"
	push_result "$name"
}

# The CPU seconds the live processes of a process group have used
group_cpu_seconds() {
	ps -eo pgid=,times= 2>/dev/null | awk -v group="$1" '$1 == group { total += $2 } END { print int(total) }'
}

# The command, its output appended to the log, killed with all its processes after TEST_TIME_LIMIT_SECONDS or once no
# test finished while they waited, using no CPU, for STALL_SECONDS on end (a hang held batch-3's session 85 min), or
# for BUSY_STALL_SECONDS at all; the summary then names the unfinished tests
run_watched() {
	local log=$1 runner finished last_finished=-1 still=0 idle=0 cpu last_cpu=0 seen started=$SECONDS; shift
	seen=$(wc -l < "$log")
	timeout --kill-after=30 $TEST_TIME_LIMIT_SECONDS "$@" >> "$log" 2>&1 &   # its own process group: a kill reaches the test binaries
	runner=$!
	while kill -0 $runner 2>/dev/null; do
		sleep $WATCH_SECONDS
		finished=$(grep -c '^test .* \.\.\. ' "$log")
		# each test's finish, to WATCH_SECONDS: libtest prints no durations
		seen=$(awk -v from="$seen" -v at=$((SECONDS - started)) -v times="$HERE_WORK/finish-times.txt" \
			'NR > from && /^test [^ ]+ \.\.\. / { print at, $2 >> times } END { print NR }' "$log")
		cpu=$(group_cpu_seconds $runner)
		if [ "$finished" != "$last_finished" ]; then still=0 idle=0 last_finished=$finished
		elif [ $((cpu - last_cpu)) -ge $BUSY_CPU_SECONDS ]; then still=$((still + WATCH_SECONDS)) idle=0
		else still=$((still + WATCH_SECONDS)) idle=$((idle + WATCH_SECONDS)); fi
		last_cpu=$cpu
		if [ $idle -ge $STALL_SECONDS ] || [ $still -ge $BUSY_STALL_SECONDS ]; then
			echo "cloud-test: KILLED: no test finished for $still s, the last $idle s of them using no CPU" >> "$log"
			kill -TERM $runner
			break
		fi
	done
	wait $runner
	[ $? -eq 124 ] && echo "cloud-test: KILLED after the time limit of $TEST_TIME_LIMIT_SECONDS s" >> "$log"
}

# Counts, failing tests with the first line of their panic, environmental and known ones (notes/known_failures.md,
# which the gate accepts too) apart; verdict green|red
summarize() {
	local log=$1 environmental=$2 known=${3:-/dev/null}
	awk -v environmental="$environmental" -v known_list="$known" -v SHOWN_ITEMS=$SHOWN_ITEMS -v NO_TESTS_RAN="$NO_TESTS_RAN" \
		-v RERUN_MARK="$RERUN_MARK" -v SHARD_HEADER="$SHARD_HEADER" '
		BEGIN {
			# a line is "<test regex>", or "<test regex> <item regex>" for a test whose panic lists items (one sample per
			# line): then only those items are environmental, and the test is environmental only if every item it lists is
			while ((getline line < environmental) > 0) {
				sub(/[ \t]*#.*/, "", line)
				if (line == "") continue
				test_pattern = line; sub(/[ \t].*/, "", test_pattern)
				item_pattern = substr(line, length(test_pattern) + 1); sub(/^[ \t]+/, "", item_pattern)
				patterns[++count] = test_pattern; item_patterns[count] = item_pattern
			}
			while ((getline line < known_list) > 0) if (line ~ /^- [^ |]+ \|/) { split(line, field, " "); known[field[2]] = 1 }
		}
		!rerun && /^test result:/ { for (i = 1; i <= NF; i++) { if ($(i) ~ /^(passed|failed|ignored|filtered)/) { key = $(i); sub(/[;,]/, "", key); total[key] += $(i - 1) } } finished++ }
		!rerun && /^ +(Running|Doc-tests) / { started++ }
		# a rerun (rerun_killed_failures) only adds panic texts; in a merged log the next shard counts again
		index($0, RERUN_MARK) == 1 { rerun = 1; next }
		index($0, SHARD_HEADER) == 1 { rerun = 0; next }
		index($0, NO_TESTS_RAN) == 1 { unmatched_line = $0 }
		!rerun && /^test [^ ]+ \.\.\. .*FAILED/ { failed[$2] = 1; order[++failures] = $2 }
		!rerun && /^test [^ ]+ has been running for over/ { slow[$2] = 1 }
		!rerun && /^test [^ ]+ \.\.\. / { finished_test[$2] = 1 }
		/^---- [^ ]+ stdout ----$/ { current = $2; want_panic = 1; next }
		collecting && /^(stack backtrace:|note: |---- |$)/ { collecting = 0 }
		collecting { items[current, ++item_count[current]] = $0 }
		want_panic && / panicked at / { getline message; panic[current] = message; want_panic = 0; collecting = 1 }
		function environmental_failure(name,    p, i, whole, listed, matched) {
			for (p = 1; p <= count; p++) if (name ~ patterns[p] && item_patterns[p] == "") whole = 1
			if (whole) return 1
			if (!item_count[name]) return 0
			for (i = 1; i <= item_count[name]; i++) {
				matched = 0
				for (p = 1; p <= count; p++) if (item_patterns[p] != "" && name ~ patterns[p] && items[name, i] ~ item_patterns[p]) matched = 1
				if (!matched) return 0
				listed = 1
			}
			return listed
		}
		END {
			incomplete = started > finished ? started - finished : 0
			for (i = 1; i <= failures; i++) {
				name = order[i]; line = "  " name ": " (name in panic ? panic[name] : "(no panic line)")
				is_environmental = environmental_failure(name)
				if (!is_environmental) for (j = 1; j <= item_count[name] && j <= SHOWN_ITEMS; j++) line = line "\n    " items[name, j]
				if (is_environmental) { environmental_lines = environmental_lines line "\n"; environmental_count++ }
				else if (name in known) { known_lines = known_lines line "\n"; known_count++ }
				else { real_lines = real_lines line "\n"; real++ }
			}
			real += incomplete
			# a run of no test is never green: filters that match nothing, or a VM whose build is not the tip
			ran = total["passed"] + failures + incomplete
			printf "verdict: %s\n", (real == 0 && started > 0 && ran > 0 && unmatched_line == "") ? "green" : "red"
			if (ran == 0) printf "%s%d tests filtered out, none ran\n", NO_TESTS_RAN, total["filtered"]
			else if (unmatched_line != "") print unmatched_line
			printf "passed: %d, failed: %d (%d environmental, %d known), ignored: %d\n", total["passed"], failures + incomplete, environmental_count, known_count, total["ignored"]
			if (incomplete) printf "crashed or killed test binaries (no result line): %d\n", incomplete
			for (name in slow) if (!(name in finished_test)) printf "  never finished: %s\n", name
			if (real_lines != "") printf "failures:\n%s", real_lines
			if (environmental_lines != "") printf "environmental (Mac-only, %s):\n%s", "scripts/cloud-test-environmental.txt", environmental_lines
			if (known_lines != "") printf "known (notes/known_failures.md at the tested tip):\n%s", known_lines
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
		git show "refs/$JOBS_PREFIX/$job:slow" > "${arguments%/*}/slow" 2>/dev/null
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
	local job=$1 arguments script slow tree commit output; shift
	arguments=$(printf '%s\n' "$@" | git -C "$MAIN_CHECKOUT" hash-object -w --stdin)
	slow=$(printf '%s' "$SLOW_FILES" | git -C "$MAIN_CHECKOUT" hash-object -w --stdin)
	script=$(git -C "$MAIN_CHECKOUT" hash-object -w --stdin < "$THIS_SCRIPT")
	tree=$(printf '100644 blob %s\targuments\n100644 blob %s\tcloud-test.sh\n100644 blob %s\tslow\n' "$arguments" "$script" "$slow" |
		git -C "$MAIN_CHECKOUT" mktree)
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
mark_session() { printf 'job: %s\nsince: %s\njobs: %s\ngate: %s\n' "$2" "$(date +%s)" "$3" "${4:-}" > "$1"; }

# A pooled session is free once its job's result is in (collected, or still on its results branch); a job that
# never answered drops its session, which may be gone
session_is_free() {
	local file=$1 job since run
	job=$(pool_field "$file" job); since=$(pool_field "$file" since)
	[ "$job" = idle ] && return 0
	run=${job%-*-*}   # a job is <run>-<launch time>-<launcher pid>; ${job%-*} kept the time: no result was ever seen, a
	                  # finished session stayed busy until collected, and was dropped as "never answered" after 90 min
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

# Under the pool lock: whether a launcher of a gate run waits for a slot (waiting-<job> holds its process id)
gate_waits() {
	local file
	for file in "$POOL"/waiting-*; do
		[ -e "$file" ] || continue
		kill -0 "$(cat "$file")" 2>/dev/null && return 0
		rm -f "$file"
	done
	return 1
}

# Under the pool lock: a free session for the job (its id, marked busy; jobs 1 = its context is to be cleared first),
# "new" when one may be started, else nothing. A gate run may take any of the MAX_PARALLEL_RUNS slots; a worker's run
# only one of WORKER_RUNS, and none while a gate run waits
claim_session() {
	local job=$1 gate=$2 file jobs free="" busy=0 worker_busy=0 sessions
	for file in "$POOL"/session_* "$POOL"/starting-*; do
		[ -e "$file" ] || continue
		if [[ "${file##*/}" == session_* ]] && session_is_free "$file"; then
			free=${free:-$file}
			continue
		fi
		[ -e "$file" ] || continue
		busy=$((busy + 1))
		[ "$(pool_field "$file" gate)" = 1 ] || worker_busy=$((worker_busy + 1))
	done
	[ $busy -lt $MAX_PARALLEL_RUNS ] || return
	[ "$gate" = 1 ] || { [ $worker_busy -lt $WORKER_RUNS ] && ! gate_waits; } || return
	if [ -n "$free" ]; then
		jobs=$(pool_field "$free" jobs)
		[ "$jobs" -ge $JOBS_PER_SESSION ] && jobs=0
		mark_session "$free" "$job" $((jobs + 1)) "$gate"; echo "${free##*/}"; return
	fi
	sessions=$(ls "$POOL" | grep -c -E '^(session_|starting-)')
	[ $sessions -lt $MAX_CLOUD_SESSIONS ] && { mark_session "$POOL/starting-$job" "$job" 1 "$gate"; echo new; }
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
	name=$(run_name "$branch" "$tip" ${base[@]+"${base[@]}"} ${settings[@]+"${settings[@]}"} "$@")
	job=$name-$(date +%s)-$$
	folder=$MAIN_CHECKOUT/data/cloud/$name
	mkdir -p "$folder"
	rm -f "$folder/result.md"
	lock_pool; session=$(pending_session_of "$name"); unlock_pool
	if [ -n "$session" ]; then
		echo "cloud-test: $name already runs in https://claude.ai/code/${session#starting-}: waiting for that result" >&2
		return
	fi
	push_job "$job" ${base[@]+"${base[@]}"} ${settings[@]+"${settings[@]}"} "$branch" "$tip" "$@"
	cloud_prompt "$job" > "$folder/prompt.txt"
	[[ "$branch" =~ $GATE_BRANCHES ]] && gate=1
	[ "$gate" = 1 ] && { echo $$ > "$POOL/waiting-$job"; trap "rm -f '$POOL/waiting-$job'" EXIT; }
	while [ -z "$url" ]; do
		lock_pool; session=$(claim_session "$job" "$gate"); unlock_pool
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
			*) echo "cloud-test: no run slot free (at most $MAX_PARALLEL_RUNS runs, $WORKER_RUNS of workers, $MAX_CLOUD_SESSIONS sessions; gate runs first), waiting for one" >&2; sleep $POLL_SECONDS ;;
		esac
	done
	rm -f "$POOL/waiting-$job"
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

finish_session() {
	local job=$1 result=$2 reason file
	reason=$(sed -n "s/^environment: //p" "$result")
	if [ -z "$reason" ]; then
		release_session "$job"
		return
	fi
	lock_pool
	for file in "$POOL"/session_*; do
		[ -e "$file" ] || continue
		[ "$(pool_field "$file" job 2>/dev/null)" = "$job" ] && drop_session "$file" "$reason"
	done
	unlock_pool
}

# Waits for run <name>'s result, saves it to data/cloud/<name>/ and hands its session the next job
fetch_result() {
	local name=$1 folder ref deadline
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
	finish_session "$(pool_field "$folder/launch.txt" job)" "$folder/result.md"
}

# Prints the result in <folder>, adds it to the card; exit status 0 = green
report() {
	local folder=$1 tip=$2 card=$3
	cat "$folder/result.md"
	echo "(log: $folder/log.txt.gz)"
	[ -n "$card" ] && todo note "$card" "cloud test $(sed -n 's/^verdict: //p' "$folder/result.md"): $(sed -n 's/^tests: //p' "$folder/result.md") at ${tip:0:8}, $(sed -n 's/^passed: //p' "$folder/result.md"); $(sed -n 's/^wall: //p' "$folder/result.md")$(sed -n '/^failures:/,/^[a-z]/{/^  /p;}' "$folder/result.md" | head -10 | tr '\n' ';')" > /dev/null
	grep -q '^verdict: green' "$folder/result.md"
}

collect() {
	local branch=$1 tip=$2 card=$3 name
	shift 3
	name=$(run_name "$branch" "$tip" ${base[@]+"${base[@]}"} ${settings[@]+"${settings[@]}"} "$@")
	fetch_result "$name"
	report "$MAIN_CHECKOUT/data/cloud/$name" "$tip" "$card"
}

# The shards' results as one: their logs one after another, summarized together; green when every shard is and all
# listed tests ran (each test in one shard: Σ passed+failed+ignored of the shards = what each shard lists)
merge_shards() {
	local folder=$1 tip=$2 shard_folder verdict=green counted listed; shift 2
	mkdir -p "$folder"
	for shard_folder in "$@"; do echo "$SHARD_HEADER${shard_folder##*/}"; gzip -dc "$shard_folder/log.txt.gz"; done > "$folder/log.txt"
	git -C "$MAIN_CHECKOUT" show "origin/main:$ENVIRONMENTAL_LIST" > "$folder/environmental.txt" 2>/dev/null
	git -C "$MAIN_CHECKOUT" show "$tip:$KNOWN_FAILURES" > "$folder/known_failures.md" 2>/dev/null
	summarize "$folder/log.txt" "$folder/environmental.txt" "$folder/known_failures.md" > "$folder/summary.txt"
	read -r counted listed < <(awk -v RERUN_MARK="$RERUN_MARK" -v SHARD_HEADER="$SHARD_HEADER" '
		index($0, SHARD_HEADER) == 1 { rerun = 0; shard++ } index($0, RERUN_MARK) == 1 { rerun = 1 }
		!rerun && /^test result:/ { for (i = 1; i <= NF; i++) { if ($(i) ~ /^(passed|failed|ignored)/) ran += $(i - 1); if ($(i) ~ /^(passed|failed|ignored|filtered)/) listed[shard] += $(i - 1) } }
		END { for (s in listed) if (listed[s] > most) most = listed[s]; print ran + 0, most + 0 }' "$folder/log.txt")
	grep -q '^verdict: green' "$folder/summary.txt" || verdict=red
	for shard_folder in "$@"; do grep -q '^verdict: green' "$shard_folder/result.md" || verdict=red; done
	[ "$counted" = "$listed" ] || verdict=red
	{
		echo "run: ${folder##*/}"
		grep -m2 '^branch: \|^tip: ' "$1/result.md"
		echo "tests: --full ($# shards)"
		echo "verdict: $verdict"
		grep -v '^verdict: ' "$folder/summary.txt"
		echo "tests counted: $counted of $listed listed$([ "$counted" = "$listed" ] || echo ": INCOMPLETE, a shard lost tests")"
		echo "wall: $(for shard_folder in "$@"; do sed -n 's/^wall: .*total \([0-9]*\) s.*/\1/p' "$shard_folder/result.md"; done | sort -n | tail -1) s, the slowest shard"
		for shard_folder in "$@"; do
			echo "shard ${shard_folder##*/}: $(sed -n 's/^verdict: //p' "$shard_folder/result.md"); $(sed -n 's/^build: //p; s/^error: //p; s/^passed: //p; s/^wall: //p; s/^last to finish: /last: /p; s/^session: //p' "$shard_folder/result.md" | tr '\n' ';')"
		done
	} > "$folder/result.md"
	gzip -f "$folder/log.txt"
}

# The test files with a test that ran over 60 s in the newest merged --full run (the shards deal them first)
slow_files() {
	local newest
	newest=$(ls -t "$MAIN_CHECKOUT"/data/cloud/*/result.md 2>/dev/null | xargs grep -l '^tests: --full (' 2>/dev/null | head -1)
	[ -n "$newest" ] || return 0
	gzip -dc "${newest%/*}/log.txt.gz" | sed -n 's/^test \([^ ]*\) has been running for over.*/\1/p' | test_files
}

collect_full() {
	local branch=$1 tip=$2 card=$3 shard name shard_names=() shard_folders=()
	shift 3
	for shard in $(seq 1 $FULL_SHARDS); do
		name=$(run_name "$branch" "$tip" ${base[@]+"${base[@]}"} ${settings[@]+"${settings[@]}"} --full $SHARD_FLAG "$shard/$FULL_SHARDS")
		shard_names+=("$name"); shard_folders+=("$MAIN_CHECKOUT/data/cloud/$name")
	done
	for name in "${shard_names[@]}"; do fetch_result "$name"; done
	name=$(run_name "$branch" "$tip" ${base[@]+"${base[@]}"} ${settings[@]+"${settings[@]}"} --full)
	merge_shards "$MAIN_CHECKOUT/data/cloud/$name" "$tip" "${shard_folders[@]}"
	report "$MAIN_CHECKOUT/data/cloud/$name" "$tip" "$card"
}

# ---------------------------------------------------------------- arguments

# one function, parsed whole before it runs: editing this file during a long wait then can't break the waiting run
main() {
	case "${1:-}" in
		--job) run_job "$2"; exit ;;
		--run-here) shift; run_here "$@"; exit ;;
		--run-detached) HERE_WORK=$HERE_ROOT/jobs/$2; shift 2; mkdir -p "$HERE_WORK"; trap 'touch "$HERE_WORK/done"' EXIT; run_tests_here "$@"; exit ;;
		--summarize) summarize "$2" "${3:-$ENVIRONMENTAL_LIST}" "${4:-}"; exit ;;
	esac
	card="" wait=1 result_only="" base=() settings=() pure_hello="" optimized="" gate=""
	while [[ "${1:-}" == --* ]]; do
		case "$1" in
			--card) card=$2; shift 2 ;;
			--no-wait) wait=""; shift ;;
			--result) result_only=1; shift ;;
			--on-main) base=(--on main); shift ;;
			--on) base=(--on "$2"); shift 2 ;;
			--env) settings+=(--env "$2"); shift 2 ;;
			--pure-hello) pure_hello=1; shift ;;
			--optimized) optimized=1; shift ;;
			--gate) gate=1; shift ;;
			*) break ;;
		esac
	done
	[ $# -ge 2 ] || { sed -n '2,18p' "$0"; exit 2; }
	if [ -n "$pure_hello" ]; then
		[ $# -eq 2 ] || fail "--pure-hello is a complete test preset and does not take filters"
		set -- "$1" "$2" "${PURE_HELLO_ARGS[@]}"
	fi
	[ -n "$optimized" ] && set -- "$1" "$2" "${OPTIMIZED_TEST_PROFILE[@]}" "${@:3}"
	local filters=() filter   # the Mac's bash 3.2 has no mapfile
	while IFS= read -r filter; do filters+=("$filter"); done < <(split_filters "${@:3}")
	set -- "$1" "$2" ${filters[@]+"${filters[@]}"}
	if [ $# -eq 3 ] && [ "$3" = --full ]; then
		local shard
		if [ -z "$result_only" ]; then
			SLOW_FILES=$(slow_files)
			for shard in $(seq 1 $FULL_SHARDS); do launch "$1" "$2" --full $SHARD_FLAG "$shard/$FULL_SHARDS" || exit; done
		fi
		[ -n "$wait$result_only" ] && collect_full "$1" "$2" "$card"
		return
	fi
	if [ -z "$result_only" ]; then launch "$@" || exit; fi
	[ -n "$wait$result_only" ] && collect "$1" "$2" "$card" "${@:3}"
}
main "$@"; exit
