#!/bin/bash
# Simplified test runner - matches IDE configuration exactly
# Creates clean test_results.txt with just pass/fail lists

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

# Match IDE: unset CARGO_TARGET_DIR to use ./target
unset CARGO_TARGET_DIR

echo "Compiling tests..."
# Match RustRover's runner injection exactly
RUNNER="target.aarch64-apple-darwin.runner=['/Applications/RustRover.app/Contents/bin/native-helper/intellij-rust-native-helper']"
FEATURES="--all-features"
cargo --offline test $FEATURES --color=always --profile test --no-fail-fast --config "$RUNNER" --no-run || exit 1

echo "Running all tests..."
cargo --offline test $FEATURES --color=always --profile test --no-fail-fast --config "$RUNNER" -- --test-threads=16 2>&1 | tee "$TEMP_FILE"

# Strip ANSI color codes so the summary greps work on the raw log
sed -i '' $'s/\033\[[0-9;]*m//g' "$TEMP_FILE"

# Count test results
TOTAL_PASSED=$(grep -a -E "^test .* \.\.\. ok$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_FAILED=$(grep -a -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_IGNORED=$(grep -a -E "^test .* \.\.\. ignored$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))
TOTAL_TESTED=$((TOTAL_PASSED + TOTAL_FAILED))


# Create clean summary file
{
	echo "=== Test Results ==="
	echo ""
	echo "PASSED:"
	grep -a -E "^test .* \.\.\. ok$" "$TEMP_FILE" | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//' | sort

	echo ""
	echo "FAILED:"
	grep -a -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//' | sort

	echo ""
	echo "SUMMARY:"
	echo "${TOTAL_IGNORED} ignored, ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_TESTED} total tested"
} > "$OUTPUT_FILE"
rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
