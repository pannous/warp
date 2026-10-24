#!/bin/bash
# The cloud test lane's summary with notes/known_failures.md (scripts/cloud-test.sh summarize): a listed failure is known,
# not red; an unlisted one still is.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
folder=$REPO_ROOT/scratch/cloud-known-probe
rm -rf "$folder"; mkdir -p "$folder"
printf '%s\n' '- wasm::test_gc::reads_text | rust-emitter | texts are byte arrays now' > "$folder/known.md"
write_log() {
	printf '%s\n' "     Running tests/main.rs (target/debug/deps/tests-def)" "test x::fine ... ok" "$@" \
		"test result: FAILED. 1 passed; $# failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s" > "$folder/log.txt"
}
summary() { "$REPO_ROOT/scripts/cloud-test.sh" --summarize "$folder/log.txt" /dev/null "$folder/known.md"; }

failures=0
check() { if grep -q "$2" <<< "$3"; then echo "ok   $1"; else echo "FAIL $1:"; echo "$3"; failures=$((failures + 1)); fi; }
write_log "test wasm::test_gc::reads_text ... FAILED"
only_known=$(summary)
check "a known failure alone is green" '^verdict: green' "$only_known"
check "counted as known" '(0 environmental, 1 known)' "$only_known"
check "and listed apart" '^known (notes/known_failures.md' "$only_known"
write_log "test wasm::test_gc::reads_text ... FAILED" "test wasm::test_gc::other ... FAILED"
with_real=$(summary)
check "an unlisted failure is still red" '^verdict: red' "$with_real"
check "and listed as a failure" '^failures:' "$with_real"
rm -rf "$folder"
[ $failures = 0 ] && echo "known probe: all ok" || { echo "known probe: $failures failed"; exit 1; }
