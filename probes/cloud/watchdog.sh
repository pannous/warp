#!/bin/bash
# The cloud test lane's stall watchdog (scripts/cloud-test.sh run_watched) on a fake test run, no cloud session: one test
# finishes, one hangs in a child process; the run is killed once nothing finished for STALL_SECONDS, the child with it,
# and the summary names the hanging test.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main "\$@"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)
STALL_SECONDS=3 WATCH_SECONDS=1
folder=$REPO_ROOT/scratch/cloud-watchdog-probe
hang_marker=4711   # the hanging child's sleep, found by ps afterwards
rm -rf "$folder"; mkdir -p "$folder"

fake_test_run="echo 'test a::one ... ok'; echo 'test b::hangs has been running for over 60 seconds'; sleep $hang_marker"
started=$SECONDS
run_watched "$folder/log.txt" bash -c "$fake_test_run"
took=$((SECONDS - started))
summarize "$folder/log.txt" /dev/null > "$folder/summary.txt"
sleep 1
survivors=$(ps -eo args | grep -c "^sleep $hang_marker$")

failures=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; failures=$((failures + 1)); fi; }
check "killed after the stall, not the time limit ($took s)" "[ $took -lt 15 ]"
check "the log says why" "grep -q 'KILLED: no test finished for 3 s' '$folder/log.txt'"
check "the hanging child is gone" "[ $survivors = 0 ]"
check "the summary names the hanging test" "grep -q 'never finished: b::hangs' '$folder/summary.txt'"
check "and is red" "grep -q '^verdict: red' '$folder/summary.txt'"
rm -rf "$folder"
[ $failures = 0 ] && echo "watchdog probe: all ok" || { cat "$folder/summary.txt" 2>/dev/null; echo "watchdog probe: $failures failed"; exit 1; }
