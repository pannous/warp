#!/bin/bash
# Simplified test runner with --no-fail-fast default
# Creates clean test_results.txt with just pass/fail lists

# Use local target dir instead of global /opt/cargo (fixes IDE/CLI cache conflicts)
unset CARGO_TARGET_DIR

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

echo "Compiling tests..."
cargo test --no-run || exit 1

echo "Running all tests..."
FAST_TIMEOUT=1 cargo test --no-fail-fast -- --test-threads=16 2>&1 | tee "$TEMP_FILE"

# Count test results
TOTAL_PASSED=$(grep -E "^test .* \.\.\. ok$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_FAILED=$(grep -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_IGNORED=$(grep -E "^test .* \.\.\. ignored$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))


# Create clean summary file
{
	echo "=== Test Results ==="
	echo ""
	echo "PASSED:"
	grep -E "^test .* \.\.\. ok$" "$TEMP_FILE" | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//'

	echo ""
	echo "FAILED:"
	grep -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//'

	echo ""
	echo "SUMMARY:"
	echo "  ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_IGNORED} ignored, ${TOTAL} total" > "$OUTPUT_FILE"
}
rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
