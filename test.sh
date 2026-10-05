#!/bin/bash
# Simplified test runner - matches IDE configuration exactly
# Creates clean test_results.txt with just pass/fail lists

[ -z "$WARP_TEST_LOCKED" ] && exec "$(dirname "$0")/tests/queue.sh" "$0" "$@"

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

# agents commit via git commit-tree (no hooks), so the probes/ layout is enforced here
"$(dirname "$0")/probes/check_layout.sh" || exit 1

# Match IDE: unset CARGO_TARGET_DIR to use ./target
unset CARGO_TARGET_DIR

echo "Compiling tests..."
COMPILE_START=$SECONDS
# Match RustRover's runner injection exactly
RUNNER="target.aarch64-apple-darwin.runner=['/Applications/RustRover.app/Contents/bin/native-helper/intellij-rust-native-helper']"
FEATURES="--all-features" # native + optimizer + ffi: the default build plus the optimizer and FFI tests
cargo --offline test $FEATURES --color=always --profile test --no-fail-fast --config "$RUNNER" --no-run || exit 1

COMPILE_SECONDS=$((SECONDS - COMPILE_START))

echo "Running all tests..."
TEST_THREADS=16
RUN_START=$SECONDS
# --report-time appends each test's duration (<1.234s>) for the slowest list; libtest takes unstable options only on
# nightly, which RUSTC_BOOTSTRAP=1 stands in for (it rebuilds nothing)
RUSTC_BOOTSTRAP=1 cargo --offline test $FEATURES --color=always --profile test --no-fail-fast --config "$RUNNER" -- --test-threads=$TEST_THREADS -Zunstable-options --report-time 2>&1 | tee "$TEMP_FILE"
RUN_SECONDS=$((SECONDS - RUN_START))

# Strip ANSI color codes so the summary greps work on the raw log
sed -i '' $'s/\033\[[0-9;]*m//g' "$TEMP_FILE"
# output with a newline can push a result onto the next line (`test x ... ` then `ok <0.003s>`): join the two
awk '/ \.\.\. $/ { printf "%s", $0; next } { print }' "$TEMP_FILE" > "$TEMP_FILE.joined" && mv "$TEMP_FILE.joined" "$TEMP_FILE"

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
rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
