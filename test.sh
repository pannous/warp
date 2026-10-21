#!/bin/bash
# Simplified test runner - matches IDE configuration exactly
# Creates clean test_results.txt with just pass/fail lists

# rustc building warp-lang at opt-level 2 (OPTIMIZED_COMPILER below) took 4.2–6.2 GB, over the queue's 4 GB default
# cap; moot once tests/queue.sh caps only tests (branch queue-prebuild)
export WARP_TEST_MEMORY_CAP_GB="${WARP_TEST_MEMORY_CAP_GB:-8}"
# the queue's time limit per test binary (600 s) is for hangs in targeted runs: the full run's tests binary takes
# 415 s on a quiet machine and was killed at 602 s under load (one-dispatch gate, 2026-10-16: 22 failures, no panics)
export WARP_TEST_TIME_LIMIT="${WARP_TEST_TIME_LIMIT:-1800}"
[ -z "$WARP_TEST_LOCKED" ] && exec "$(dirname "$0")/tests/queue.sh" "$0" "$@"

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

# agents commit via git commit-tree (no hooks), so the probes/ layout is enforced here
"$(dirname "$0")/probes/check_layout.sh" || exit 1

# Match IDE: unset CARGO_TARGET_DIR to use ./target
unset CARGO_TARGET_DIR

echo "Compiling tests..."
COMPILE_START=$SECONDS
# the test executables run through a plain pass-through (RustRover's intellij-rust-native-helper until 2026-10-20)
RUNNER_PROGRAM="$PWD/tests/runner.sh"
FEATURES="--all-features" # native + ffi: the default build plus the FFI tests
# the compiler at opt-level 2 for the full suite: the tests' time is mostly warp lowering programs and their std modules;
# the tests binary ran 2x faster (summed test time 14424 → 7059 s), the build takes ~130 s longer (notes/test_speed.md).
# Debug assertions and overflow checks stay on. Gate only: workers' targeted runs keep opt-level 0, which builds faster
OPTIMIZED_COMPILER="profile.test.package.warp-lang.opt-level=2"
# no incremental build at opt-level 2: reusing the previous gate's codegen units left warp's own functions undefined at
# the lib tests' link (`precompiled_for_tasks`, `….llvm.<hash>`, twice in integrate 2026-10-16); the gate's build
# changes most of warp anyway
export CARGO_INCREMENTAL=0
TEST_EXECUTABLES=$(mktemp)
cargo --offline test $FEATURES --color=always --profile test --no-fail-fast --config "$OPTIMIZED_COMPILER" --no-run --message-format=json-render-diagnostics |
	jq -r 'select(.reason == "compiler-artifact" and .profile.test and .executable != null) | "\(.target.src_path)\t\(.executable)"' > "$TEST_EXECUTABLES"
[ "${PIPESTATUS[0]}" -eq 0 ] || exit 1
# the runtime stubs the executable tests copy (tests/common runtime_stub): built here, so no test runs cargo, whose lock
# on the shared target dir made 14 tests wait ~175 s each while other worktrees built (notes/test_speed.md)
cargo --offline build --quiet -p warp-runtime --bin warp-runtime || exit 1
cargo --offline build --quiet -p warp-runtime --bin warp-page-runtime --features webview || exit 1
export WARP_TEST_STUBS_BUILT=1

COMPILE_SECONDS=$((SECONDS - COMPILE_START))

echo "Running all tests..."
TEST_THREADS=16
RUN_START=$SECONDS
# the [env] of .cargo/config.toml as cargo gives it to the programs it runs: a value relative to the config's folder
# becomes absolute, a variable already set is kept unless the entry says force
cargo_config_env() {
	python3 -I -c 'import os, tomllib
for name, entry in tomllib.load(open(".cargo/config.toml", "rb")).get("env", {}).items():
    entry = entry if isinstance(entry, dict) else {"value": entry}
    value = os.path.abspath(entry["value"]) if entry.get("relative") else entry["value"]
    if entry.get("force") or name not in os.environ:
        print(f"export {name}={value!r}")'
}
# the test executables run directly, as cargo test would run them (through the runner, in the package folder, with
# CARGO and the config's [env] set): a second `cargo test` only waited 18–47 s for the shared target dir's lock.
# --report-time appends each test's duration (<1.234s>) for the slowest list; libtest takes unstable options only on
# nightly, which RUSTC_BOOTSTRAP=1 stands in for
eval "$(cargo_config_env)" || exit 1
run_test_executables() {
	local source executable
	while IFS=$'\t' read -r source executable; do
		echo "     Running ${source#"$PWD/"} ($executable)"
		RUSTC_BOOTSTRAP=1 CARGO="$(command -v cargo)" "$RUNNER_PROGRAM" "$executable" --test-threads=$TEST_THREADS \
			-Zunstable-options --report-time
	done < "$TEST_EXECUTABLES"
}
run_test_executables 2>&1 | tee "$TEMP_FILE"
RUN_SECONDS=$((SECONDS - RUN_START))

# a test binary that crashed or was killed prints neither its failures' panics nor which test brought it down: its
# unfinished and failed tests run again, in JSON, whose `started` events name the tests running at a second crash and
# whose `failed` events carry the panic at once (report: tests/crash_report.py)
RERUN_FILE=$(mktemp)
rerun_incomplete_executables() {
	local source executable names
	while IFS=$'\t' read -r source executable; do
		awk -v header="($executable)" 'index($0, header) { inside = 1; next } /^ +Running / { inside = 0 } inside' \
			"$TEMP_FILE" > "$TEMP_FILE.section"
		grep -q '^test result:' "$TEMP_FILE.section" && continue
		names=$("$RUNNER_PROGRAM" "$executable" --list --format terse 2>/dev/null | sed -n 's/: test$//p' |
			grep -vxF -f <(grep -a -o -E '^test [A-Za-z0-9_:]+ \.\.\. (ok|ignored)' "$TEMP_FILE.section" | cut -d' ' -f2))
		echo "RERUN of ${source#"$PWD/"}: $(echo "$names" | grep -c .) unfinished or failed tests"
		[ -z "$names" ] && continue
		# shellcheck disable=SC2086 # one argument per test name
		RUSTC_BOOTSTRAP=1 CARGO="$(command -v cargo)" "$RUNNER_PROGRAM" "$executable" --exact --test-threads=$TEST_THREADS \
			-Zunstable-options --format json --report-time $names 2>&1 | python3 -I tests/crash_report.py
	done < "$TEST_EXECUTABLES"
	rm -f "$TEMP_FILE.section"
}

# Strip ANSI color codes so the summary greps work on the raw log
sed -i '' $'s/\033\[[0-9;]*m//g' "$TEMP_FILE"
# output with a newline can push a result onto the next line (`test x ... ` or `test x ... 3`, then `ok <0.003s>`):
# a test line without its time (every result but ignored carries one) waits for the next line that ends in a time,
# lines between (another test's output, e.g. a printed tree) pass through
awk '
	/test [A-Za-z0-9_:]+ \.\.\. / { if (pending != "") print pending; pending = "" }
	/test [A-Za-z0-9_:]+ \.\.\. / && !/<[0-9.]+s>$/ && !/\.\.\. ignored/ { pending = $0; next }
	pending != "" && /<[0-9.]+s>$/ { print pending $0; pending = ""; next }
	{ print }
	END { if (pending != "") print pending }' "$TEMP_FILE" > "$TEMP_FILE.joined" && mv "$TEMP_FILE.joined" "$TEMP_FILE"
rerun_incomplete_executables | tee "$RERUN_FILE"

# Count test results
# a test's own output (a wasm program writing to stdout) can glue onto its "test … ok" line: match the test anywhere
# or around its result (`... ok2`, `... ok      <0.008s>`, `... ----::::ok <0.001s>`); `ignored, <reason>` lines stay uncounted
results() { grep -a -o -E "test [A-Za-z0-9_:]+ \.\.\. (.*[^A-Za-z])?$1($|[^,])" "$TEMP_FILE" | sed -E "s/^(test [^ ]+) \.\.\. .*/\1 ... $1/"; }
SLOWEST_COUNT=15
TIMES_FILE="data/test_times.txt" # every test's seconds, slowest first (data/ is not tracked)
durations() { grep -a -o -E "test [A-Za-z0-9_:]+ \.\.\. [A-Za-z]+ <[0-9.]+s>$" "$TEMP_FILE" | sed -E 's/^test ([^ ]+) .* <([0-9.]+)s>$/\2 \1/' | sort -rn; }
slowest() { durations | head -$SLOWEST_COUNT; }
TOTAL_PASSED=$(results ok | wc -l | tr -d ' ')
TOTAL_FAILED=$(results FAILED | wc -l | tr -d ' ')
TOTAL_IGNORED=$(results ignored | wc -l | tr -d ' ')
# a test binary that crashed or was killed (a hang ended by hand) prints no `test result:` line: its unfinished tests
# would just be missing from the counts, so each such binary counts as a failure
STARTED=$(grep -a -c -E '^ +(Running|Doc-tests) ' "$TEMP_FILE")
FINISHED=$(grep -a -c '^test result:' "$TEMP_FILE")
INCOMPLETE=$((STARTED > FINISHED ? STARTED - FINISHED : 0))
TOTAL_FAILED=$((TOTAL_FAILED + INCOMPLETE))
TOTAL=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))
TOTAL_TESTED=$((TOTAL_PASSED + TOTAL_FAILED))


# Create clean summary file
{
	echo "=== Test Results ==="
	echo ""
	echo "PASSED:"
	results ok | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//' | sort

	echo ""
	echo "FAILED:"
	results FAILED | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//' | sort
	[ "$INCOMPLETE" -gt 0 ] && echo "  ✗ INCOMPLETE: $INCOMPLETE test binaries ended without a result (crashed or killed)"
	sed 's/^/    /' "$RERUN_FILE"

	echo ""
	echo "SUMMARY:"
	echo "${TOTAL_IGNORED} ignored, ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_TESTED} total tested"
} > "$OUTPUT_FILE"
# timings differ every run: printed, not saved, so test_results.txt changes only when results do
mkdir -p "$(dirname "$TIMES_FILE")"
durations > "$TIMES_FILE"
TEST_SECONDS=$(awk '{total += $1} END {printf "%.0f", total}' "$TIMES_FILE")
# the run's wall clock against the tests' summed time per thread: close means the threads were busy to the end
echo "TIMING: compile ${COMPILE_SECONDS} s, run ${RUN_SECONDS} s; tests summed ${TEST_SECONDS} s ($((TEST_SECONDS / TEST_THREADS)) s per thread, all in $TIMES_FILE); slowest tests (seconds):"
slowest | sed 's/^/  /'
rm "$TEMP_FILE" "$RERUN_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
