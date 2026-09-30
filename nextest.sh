#!/bin/bash
# Test runner using cargo-nextest for faster parallel execution

OUTPUT_FILE="${1:-test_results.txt}"
TEMP_FILE=$(mktemp)

unset CARGO_TARGET_DIR

FEATURES="--all-features"
echo "Compiling all tests..."
cargo --offline nextest run $FEATURES --no-run || exit 1

echo "Running all tests..."
cargo --offline nextest run $FEATURES 2>&1 | tee "$TEMP_FILE"

# Count test results (nextest output format)
TOTAL_PASSED=$(grep -E "PASS \[" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_FAILED=$(grep -E "FAIL \[" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL_IGNORED=$(grep -E "SKIP \[" "$TEMP_FILE" | wc -l | tr -d ' ')
TOTAL=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))
TOTAL_TESTED=$((TOTAL_PASSED + TOTAL_FAILED))

# Create clean summary file
{
	echo "=== Test Results (nextest) ==="
	echo ""
	echo "PASSED:"
	grep -E "PASS \[" "$TEMP_FILE" | sed 's/.*PASS \[.*\] /  ✓ /'
	echo ""
	echo "FAILED:"
	grep -E "FAIL \[" "$TEMP_FILE" | sed 's/.*FAIL \[.*\] /  ✗ /'
	echo ""
	echo "SUMMARY:"
	echo "  ${TOTAL_IGNORED} ignored, ${TOTAL_PASSED} passed, ${TOTAL_FAILED} failed, ${TOTAL_TESTED} total tested"
} > "$OUTPUT_FILE"

rm "$TEMP_FILE"

echo ""
echo "Clean summary saved to: $OUTPUT_FILE"
cat "$OUTPUT_FILE"
