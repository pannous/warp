#!/bin/bash
# The cloud test lane's pool rules (scripts/cloud-test.sh claim_session) on a scratch pool, no cloud session started:
# reuse first, a session at JOBS_PER_SESSION is reused with its count reset (to be /clear-ed), new ones only below
# MAX_PARALLEL_RUNS busy and MAX_CLOUD_SESSIONS in total.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main "\$@"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)
POOL=$REPO_ROOT/scratch/cloud-pool-probe
DROPPED_SESSIONS=$POOL.dropped
failures=0

reset_pool() { rm -rf "$POOL"; mkdir -p "$POOL"; }
busy_sessions() { local i; for i in $(seq 1 "$1"); do mark_session "$POOL/session_busy$i" "nojob-probe-$i" 1; done; }
expect() {
	local name=$1 wanted=$2 got=$3
	if [ "$got" = "$wanted" ]; then echo "ok   $name"; else echo "FAIL $name: wanted '$wanted', got '$got'"; failures=$((failures + 1)); fi
}

reset_pool
expect "empty pool starts a session" new "$(claim_session job-a)"

reset_pool; mark_session "$POOL/session_idle" idle 5
expect "an idle session is reused" session_idle "$(claim_session job-b)"
expect "its job count grows" 6 "$(pool_field "$POOL/session_idle" jobs)"

reset_pool; mark_session "$POOL/session_old" idle $JOBS_PER_SESSION
expect "a session at the job limit is reused" session_old "$(claim_session job-c)"
expect "with its count reset, so launch clears it" 1 "$(pool_field "$POOL/session_old" jobs)"

reset_pool; busy_sessions $MAX_PARALLEL_RUNS
expect "MAX_PARALLEL_RUNS busy: wait" "" "$(claim_session job-d)"

MAX_PARALLEL_RUNS=20
reset_pool; busy_sessions $((MAX_CLOUD_SESSIONS - 1))
expect "below MAX_CLOUD_SESSIONS: start one" new "$(claim_session job-e)"
reset_pool; busy_sessions $MAX_CLOUD_SESSIONS
expect "MAX_CLOUD_SESSIONS exist: wait" "" "$(claim_session job-f)"

rm -rf "$POOL" "$DROPPED_SESSIONS"
[ $failures = 0 ] && echo "pool probe: all ok" || { echo "pool probe: $failures failed"; exit 1; }
