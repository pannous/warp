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
rm -r "$folder"
