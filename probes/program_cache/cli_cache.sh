#!/bin/bash
# Probe for card cache-compiled-shader: `warp <file>` runs twice from one kept program; a changed program or a changed
# file a `use` reads compiles again. Usage: probes/program_cache/cli_cache.sh [warp binary] (default scratch/warp)
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
warp="${1:-$repo/scratch/warp}"
work="$repo/scratch/program_cache_probe"
rm -rf "$work" && mkdir -p "$work/cache" && cd "$work"
export WARP_MODULE_CACHE="$work/cache" WARP_NO_WINDOW=1
run() { "$warp" --no-ask run program.warp; }
programs() { ls cache | grep -c '\.program$' || true; }
expect() { [ "$1" = "$2" ] || { echo "FAIL: $3: got '$1', expected '$2'"; exit 1; }; }

printf 'square(x) := x * x\nprint square(7)' >program.warp
expect "$(run)" 49 "first run"
expect "$(programs)" 1 "kept after first run"
expect "$(run)" 49 "second run"
expect "$(programs)" 1 "second run hits"
printf 'square(x) := x * x\nprint square(8)' >program.warp
expect "$(run)" 64 "changed program"
expect "$(programs)" 2 "changed program kept apart"

printf 'use words\nprint bump(1)' >program.warp
printf 'bump(x) := x + 1' >words.warp
expect "$(run)" 2 "used file"
expect "$(programs)" 3 "used file kept"
printf 'bump(x) := x + 10' >words.warp
expect "$(run)" 11 "changed used file"

mkdir -p sub && printf 'bump(x) := x + 100' >sub/words.warp
printf 'use sub/words\nprint bump(1)' >program.warp
before="$(programs)"
expect "$(run)" 101 "subfolder module"
expect "$(programs)" "$before" "subfolder module not kept"
echo "program cache probe: ok"
