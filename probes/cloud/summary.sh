#!/bin/bash
# The cloud test lane's summary of a cargo test log (scripts/cloud-test.sh summarize): run it, expect red, 1 real
# failure (a::two), 1 environmental (sound::test_play::plays), 1 crashed binary (other.rs).
folder=$(mktemp -d "${TMPDIR:-/tmp}/cloud-summary.XXXX")
cat > "$folder/log.txt" <<'LOG'
     Running unittests src/lib.rs (target/debug/deps/warp-abc)
running 2 tests
test a::one ... ok
test a::two ... FAILED

failures:

---- a::two stdout ----

thread 'a::two' panicked at src/a.rs:3:5:
assertion `left == right` failed
  left: 1

failures:
    a::two

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/main.rs (target/debug/deps/tests-def)
running 3 tests
test sound::test_play::plays ... FAILED
test x::y ... ok
test x::z ... ignored
---- sound::test_play::plays stdout ----
thread 'sound::test_play::plays' panicked at tests/sound/test_play.rs:9:1:
no audio device
test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/other.rs (target/debug/deps/other-123)
LOG
echo 'sound::test_play  # no audio device on Linux' > "$folder/environmental.txt"
"$(dirname "$0")/../../scripts/cloud-test.sh" --summarize "$folder/log.txt" "$folder/environmental.txt"
# filters that match no test: red, never green (card cloud-test-lane)
cat > "$folder/none.txt" <<'LOG'
     Running tests/main.rs (target/debug/deps/tests-def)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6642 filtered out; finished in 0.00s
LOG
none=$("$(dirname "$0")/../../scripts/cloud-test.sh" --summarize "$folder/none.txt" "$folder/environmental.txt")
[[ "$none" == "verdict: red"*"NO TESTS RAN: 6642 tests filtered out"* ]] || { echo "summary probe: FAIL, a run of no test is not red: $none"; exit 1; }
# a filter no built test matched, though others ran: red too
cat > "$folder/partial.txt" <<'LOG'
NO TESTS RAN: filters matching no test after a fresh build: functions::test_effects::
     Running tests/main.rs (target/debug/deps/tests-def)
running 1 tests
test x::y ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6674 filtered out; finished in 0.01s
LOG
partial=$("$(dirname "$0")/../../scripts/cloud-test.sh" --summarize "$folder/partial.txt" "$folder/environmental.txt")
[[ "$partial" == "verdict: red"*"functions::test_effects::"* ]] || { echo "summary probe: FAIL, an unmatched filter is not red: $partial"; exit 1; }
rm -r "$folder"
