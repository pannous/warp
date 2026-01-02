#!/bin/bash
# Simplified test runner with --no-fail-fast default
# Creates clean test_results.txt with just pass/fail lists

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

echo "Running all tests..."
cargo test --no-fail-fast 2>&1 | tee "$TEMP_FILE"

# Count test results
TOTAL_PASSED=$(grep -E "^test .* \.\.\. ok$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_FAILED=$(grep -E "^test .* \.\.\. FAILED$" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_IGNORED=$(grep -E "^test .* \.\.\. ignored$" "$TEMP_FILE" | wc -l | tr -d ' ')

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
	echo "  ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_IGNORED} ignored"
} > "$OUTPUT_FILE"

rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
