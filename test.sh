#!/bin/bash
# Simplified test runner with --no-fail-fast default
# Creates clean test_results.txt with just pass/fail lists

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

echo "Compiling tests... (profile: fast)"
# Match RustRover flags exactly for fingerprint consistency
RUNNER="target.aarch64-apple-darwin.runner=['/Applications/RustRover.app/Contents/bin/native-helper/intellij-rust-native-helper']"
cargo --offline test --color=always --profile fast --no-fail-fast --config "$RUNNER" --no-run || exit 1

echo "Running all tests..."
FAST_TIMEOUT=1 cargo --offline test --color=always --profile fast --no-fail-fast --config "$RUNNER" -- --test-threads=16 -Z unstable-options --show-output 2>&1 | tee "$TEMP_FILE"

# Count test results
TOTAL_PASSED=$(grep -E "^test .* \.\.\. ok$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_FAILED=$(grep -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_IGNORED=$(grep -E "^test .* \.\.\. ignored$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))
TOTAL_TESTED=$((TOTAL_PASSED + TOTAL_FAILED))


# Create clean summary file
{
	echo "=== Test Results ==="
	echo ""
	echo "PASSED:"
	grep -E "^test .* \.\.\. ok$" "$TEMP_FILE" | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//' | tee $OUTPUT_FILE

	echo ""
	echo "FAILED:"
	grep -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//' | tee $OUTPUT_FILE

	echo ""
	echo "SUMMARY:"
	echo "${TOTAL_IGNORED} ignored, "
	echo "  ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_TESTED} total tested" > "$OUTPUT_FILE"
}
rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
