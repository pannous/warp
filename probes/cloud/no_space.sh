#!/bin/bash
# A cloud build that is still out of space after its clean retry is environmental: its VM leaves the pool and the
# result says why without blaming the tested branch.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main \"\$@\"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)
folder=$REPO_ROOT/scratch/cloud-no-space-probe
POOL=$folder/pool
DROPPED_SESSIONS=$folder/dropped-sessions.txt
mkdir -p "$POOL"

failures=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; failures=$((failures + 1)); fi; }
printf 'error: failed to write output: No space left on device (os error 28)\n' > "$folder/log.txt"
environment=$(detect_build_environment 101 "$folder/log.txt")
check "disk exhaustion is a build environment failure" "[ '$environment' = 'disk exhausted after a clean rebuild' ]"
write_summary "$folder/log.txt" /dev/null /dev/null "$environment" > "$folder/result.md"
check "the verdict is environmental" "grep -q '^verdict: environmental$' '$folder/result.md'"
check "the result says why" "grep -q '^environment: disk exhausted after a clean rebuild$' '$folder/result.md'"

job=some-branch-0123abcd-1792000000
mark_session "$POOL/session_full" "$job" 7
finish_session "$job" "$folder/result.md"
check "the full session leaves the pool" "[ ! -e '$POOL/session_full' ]"
check "the dropped-session log says why" "grep -q 'disk exhausted after a clean rebuild' '$DROPPED_SESSIONS'"

find "$folder" -type f -delete
find "$folder" -depth -type d -delete
[ $failures = 0 ] && echo "no-space probe: all ok" || { echo "no-space probe: $failures failed"; exit 1; }
