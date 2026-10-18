#!/bin/bash
# The cloud test lane's disk guard (scripts/cloud-test.sh make_room) on a fake target/, no cloud session: with room it
# keeps target/; short of room it drops the incremental caches, then all of target/.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main "\$@"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)
folder=$REPO_ROOT/scratch/cloud-disk-probe
fake_target() { rm -rf "$folder"; mkdir -p "$folder/target/debug/incremental/warp-x" "$folder/target/debug/deps"; touch "$folder/target/debug/deps/libwarp.rlib"; }

failures=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; failures=$((failures + 1)); fi; }
fake_target; cd "$folder"
MIN_FREE_GB=0 said=$(make_room)
check "with room target/ stays ($said)" "[ -e target/debug/incremental/warp-x ] && [ -e target/debug/deps/libwarp.rlib ]"
MIN_FREE_GB=1000000 said=$(make_room)
check "short of room everything goes ($said)" "[ ! -e target ]"
check "and it says so" "grep -q 'after dropping target/' <<< '$said'"
cd "$REPO_ROOT"; rm -rf "$folder"
[ $failures = 0 ] && echo "disk probe: all ok" || { echo "disk probe: $failures failed"; exit 1; }
