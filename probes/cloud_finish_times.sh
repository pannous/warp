#!/bin/bash
# scripts/cloud-test.sh's watchdog notes when each test finishes (libtest prints no durations) and the result names
# the last to finish (card cloud-lane-sharded). Works in scratch/finish-times/
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
eval "$(sed '$d' "$root/scripts/cloud-test.sh")"
HERE_WORK=$root/scratch/finish-times WATCH_SECONDS=1
mkdir -p "$HERE_WORK" && : > "$HERE_WORK/log.txt" && : > "$HERE_WORK/finish-times.txt"
run_watched "$HERE_WORK/log.txt" bash -c 'sleep 1.5; echo "test a::b::c ... ok"; sleep 2; echo "test a::b::d ... ok"; echo "test a::e::f ... FAILED"; sleep 1'
cat "$HERE_WORK/finish-times.txt"   # 2 a::b::c, then 4 a::b::d and 4 a::e::f (to the second)
