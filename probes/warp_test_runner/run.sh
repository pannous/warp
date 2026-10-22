#!/bin/bash
# `warp test <folder>` (card warp-test-runner): the folder's pairs and is! files run through `warp pure`, one child each;
# cases/ holds 8 tests, one deliberately wrong and one skipped
WARP=${WARP:-warp}
cd "$(dirname "$0")"
output=$(WARP_NO_WINDOW=1 WARP_HINTS=0 "$WARP" test cases)
status=$?
echo "$output"
expected_failure='✗ cases/sample.pairs.warp:3 "1+1" 3: got 2'
expected_total='1 of 8 failed in 2 files, 1 skipped'
grep -qxF "$expected_failure" <<<"$output" || { echo "FAIL: no line $expected_failure"; exit 1; }
[[ $(tail -1 <<<"$output") == "$expected_total" ]] || { echo "FAIL: total is not $expected_total"; exit 1; }
[[ $status == 1 ]] || { echo "FAIL: exit $status, not 1"; exit 1; }
echo "PASS"
