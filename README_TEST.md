# Test Result Logging

## Quick Usage

### Simple tee output (recommended for daily use)
```bash
./quicktest.sh
```
Saves results to `test_results.txt` while displaying on screen.

### Full test report with summary
```bash
./test.sh [output_file]
```
Creates detailed output:
- `test_results.txt` - Full human readable output
- `test_results_summary.txt` - Concise summary with pass/fail lists

### Manual commands

#### Basic tee (shows and saves)
```bash
cargo test 2>&1 | tee test_results.txt
```

#### JSON format (requires nightly Rust)
```bash
cargo +nightly test -- --format=json -Z unstable-options 2>&1 | tee test_results.json
```

#### Get list of all tests with pass/fail (with jq)
```bash
cargo +nightly test -- --format=json -Z unstable-options | jq -r 'select(.type == "test") | "\(.name): \(.event)"'
```

#### Terse format (one char per test)
```bash
cargo test -- --format=terse 2>&1 | tee test_results.txt
```

## Format Options

- `--format=pretty` - Verbose output (default)
- `--format=terse` - One character per test (. = pass, F = fail)
- `--format=json` - Machine-readable JSON
- `--format=junit` - JUnit XML format

## Parsing Results

### With jq (install via: brew install jq)
```bash
# List all tests with status
jq -r 'select(.type == "test") | "\(.name): \(.event)"' test_results.json

# Count passed tests
jq -r 'select(.type == "test" and .event == "ok") | .name' test_results.json | wc -l

# List only failed tests
jq -r 'select(.type == "test" and .event == "failed") | .name' test_results.json
```

### Without jq
```bash
# Count tests
grep -E "test result:" test_results.txt

# List failed tests
grep "FAILED" test_results.txt
```
