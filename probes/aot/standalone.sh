#!/bin/bash
# Size and start-up of a standalone executable: `warp build --exe` with the release warp-runtime stub (notes/aot.md).
# usage: probes/aot/standalone.sh <program.wasp> [warp binary]     (run from the repo root)
set -e
SOURCE=${1:?program.wasp}
WARP=${2:-scratch/warp}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TARGET=$(cd "$ROOT" && cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
(cd "$ROOT" && cargo build --offline --release -p warp-runtime 2>&1 | tail -1)
STUB=$ROOT/scratch/warp-runtime
cp "$TARGET/release/warp-runtime" "$STUB"
WARP_RUNTIME_STUB=$STUB "$WARP" build --exe "$SOURCE" | tail -1
EXECUTABLE=${SOURCE%.*}.exe
echo "stub $(stat -f %z "$STUB") B, executable $(stat -f %z "$EXECUTABLE") B"
for _ in 1 2 3; do /usr/bin/time -p "$EXECUTABLE" 2>&1 | grep real; done
