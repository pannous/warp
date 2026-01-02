#!/bin/bash
# Test runner that saves results to file while displaying on screen

# Default output file
OUTPUT_FILE="${1:-test_results.txt}"
SUMMARY_FILE="${OUTPUT_FILE%.txt}_summary.txt"

echo "Running tests..."
echo "Results will be saved to: $OUTPUT_FILE"

# Run tests with pretty output, tee to file
cargo test 2>&1 | tee "$OUTPUT_FILE"

# Generate summary
echo ""
echo "=== Generating Test Summary ==="
echo "Test Summary - $(date)" > "$SUMMARY_FILE"
echo "" >> "$SUMMARY_FILE"

# Extract overall results
echo "Overall Results:" >> "$SUMMARY_FILE"
grep -E "test result:" "$OUTPUT_FILE" >> "$SUMMARY_FILE"

echo "" >> "$SUMMARY_FILE"
echo "Passed Tests:" >> "$SUMMARY_FILE"
grep -E "^test .* \.\.\. ok$" "$OUTPUT_FILE" | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//' >> "$SUMMARY_FILE"

echo "" >> "$SUMMARY_FILE"
echo "Failed Tests:" >> "$SUMMARY_FILE"
grep -E "^test .* \.\.\. FAILED$" "$OUTPUT_FILE" | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//' >> "$SUMMARY_FILE"

echo "" >> "$SUMMARY_FILE"
echo "Ignored Tests:" >> "$SUMMARY_FILE"
grep -E "^test .* \.\.\. ignored$" "$OUTPUT_FILE" | sed 's/test /  - /' | sed 's/ \.\.\. ignored$//' >> "$SUMMARY_FILE"

# Display summary
cat "$SUMMARY_FILE"

echo ""
echo "Summary saved to: $SUMMARY_FILE"
