echo "Compiling all tests..."
cargo --offline nextest run --test-threads=16 --no-run || exit 1

echo "Running all tests..."
cargo --offline nextest run --test-threads=16 2>&1 | tee "$TEMP_FILE"
