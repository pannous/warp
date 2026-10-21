#!/bin/bash
# Gate runs (batch-*/express*/--gate) may take every run slot and go first; workers' runs hold at most WORKER_RUNS.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main \"\$@\"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"
POOL=$REPO_ROOT/scratch/gate_priority_pool
rm -rf "$POOL"; mkdir -p "$POOL"
session_is_free() { [ "$(pool_field "$1" job)" = idle ]; }
busy() { for i in $(seq 1 "$1"); do mark_session "$POOL/session_$2$i" "job-$2$i" 1 "$3"; done; }
check() { [ "$2" = "$3" ] && echo "ok: $1" || { echo "FAIL: $1 (expected '$3', got '$2')"; failed=1; }; }
failed=""

busy $WORKER_RUNS w ""
check "a worker run waits when workers hold their $WORKER_RUNS slots" "$(claim_session job-x "")" ""
check "a gate run starts a session then" "$(claim_session job-g 1)" new
rm -f "$POOL"/starting-*
busy $((MAX_PARALLEL_RUNS - WORKER_RUNS)) g 1
check "a gate run waits when all $MAX_PARALLEL_RUNS slots run" "$(claim_session job-g 1)" ""
mark_session "$POOL/session_idle" idle 3
rm -f "$POOL/session_w1"
check "a worker run takes the idle session below its share" "$(claim_session job-y "")" session_idle
mark_session "$POOL/session_idle" idle 3
sleep 300 & waiter=$!
echo $waiter > "$POOL/waiting-job-gate"
check "a worker run waits while a gate run waits" "$(claim_session job-z "")" ""
check "the waiting gate run takes the idle session" "$(claim_session job-gate 1)" session_idle
kill $waiter
rm -rf "$POOL"
[ -z "$failed" ] && echo "gate priority probe: ok" || exit 1
