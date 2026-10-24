#!/bin/bash
# tests/queue.sh kills a test binary over WARP_TEST_TIME_LIMIT and names it and the program it waits for (card
# tests-queue). A fake test binary (deps/<name>-<16 hex>) starts a long sleep; a private lock keeps it off the real queue.
cd "$(dirname "$0")/.." || exit 1
FAKE_DEPS=scratch/queue_time_limit/deps
FAKE_BINARY=$FAKE_DEPS/tests-0123456789abcdef
mkdir -p "$FAKE_DEPS"
ln -sf /bin/bash "$FAKE_BINARY" # run directly, as cargo's are: its path is the first word of its command

started=$SECONDS
report=$(WARP_TEST_TIME_LIMIT=3 WARP_TEST_LOCK="$PWD/scratch/queue_time_limit/lock" tests/queue.sh "$FAKE_BINARY" -c "sleep 60; true" 2>&1)
status=$?
echo "$report"
fail() { echo "FAIL: $1"; exit 1; }
[ $status -ne 0 ] || fail "the killed run exited 0"
[ $((SECONDS - started)) -lt 20 ] || fail "not killed in time ($((SECONDS - started)) s)"
grep -q "KILLED the test binary .*: $FAKE_BINARY -c" <<< "$report" || fail "the binary is not named"
[ "$(grep -c "KILLED" <<< "$report")" = 1 ] || fail "more than the test binary killed"
grep -q "waiting for: sleep 60" <<< "$report" || fail "the program it waited for is not named"
echo "PASS: killed after $((SECONDS - started)) s"
