#!/bin/bash
# card probes-printed: a run whose warp binary is rebuilt while it compiles must not keep its module under the new
# binary's key, or the rebuilt warp's first runs print the old compiler's output (parameter_named.warp printed Y).
# Simulates the rebuild by touching a private copy of the binary halfway through a slow compile, then runs again:
# the second run must compile again (two kept programs), not reuse the first run's module (one).
# Usage: probes/program_cache_rebuild_race.sh [warp binary]   (default: scratch/warp from scripts/own-warp.sh)
set -euo pipefail

FUNCTIONS=300
TOUCH_AFTER_SECONDS=0.3

repo="$(cd "$(dirname "$0")/.." && pwd)"
binary="${1:-$repo/scratch/warp}"
folder="$repo/scratch/program_cache_rebuild_race"
rm -rf "$folder"
mkdir -p "$folder/cache"
cp "$binary" "$folder/warp"
for i in $(seq 1 $FUNCTIONS); do echo "step$i(x) := x + $i"; done >"$folder/program.warp"
echo "print step$FUNCTIONS(1)" >>"$folder/program.warp"

run() { (cd "$folder" && WARP_MODULE_CACHE="$folder/cache" WARP_NO_WINDOW=1 ./warp --no-ask run program.warp); }
kept_programs() { find "$folder/cache" -name '*.program' | wc -l | tr -d ' '; }

"$folder/warp" version >/dev/null 2>&1 # macOS checks a new binary on its first start, which would outlast the sleep
run >/dev/null &
sleep $TOUCH_AFTER_SECONDS
touch "$folder/warp" # the "rebuild": a new stamp for the binary file
wait
run >/dev/null
echo "kept programs: $(kept_programs) (2 = fixed, 1 = the first run's module was kept under the rebuilt binary's key)"
[ "$(kept_programs)" = 2 ]
