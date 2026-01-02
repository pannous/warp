#!/bin/bash
# Simplified test runner with --no-fail-fast default

OUTPUT_FILE="${1:-test_results.txt}"

echo "Running all tests..."
cargo test --no-fail-fast 2>&1 | tee "$OUTPUT_FILE"

echo ""
echo "=== Test Summary ==="
echo ""
echo "Passed:"
grep -E "^test .* \.\.\. ok$" "$OUTPUT_FILE" | sed 's/test /  ✓ /' | sed 's/ \.\.\. ok$//'

echo ""
echo "Failed:"
grep -E "^test .* \.\.\. FAILED$" "$OUTPUT_FILE" | sed 's/test /  ✗ /' | sed 's/ \.\.\. FAILED$//'

echo ""
grep -E "test result:" "$OUTPUT_FILE"
