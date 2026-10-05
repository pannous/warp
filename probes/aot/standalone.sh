#!/bin/bash
# Prove the compiler-less warp-runtime loads a program's .cwasm (notes/aot.md).
# usage: probes/aot/standalone.sh <program.wasp> [warp binary]     (run from the repo root)
set -e
SOURCE=${1:?program.wasp}
WARP=${2:-scratch/warp}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TARGET=$(cd "$ROOT" && cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
(cd "$ROOT" && cargo build --offline -p warp-runtime 2>&1 | tail -1)
"$WARP" compile --aot "$SOURCE" >/dev/null
CWASM=$(cd "$(dirname "$SOURCE")" && pwd)/$(basename "${SOURCE%.*}").cwasm
export WARP_CWASM="$CWASM"
echo "cwasm $(stat -f %z "$CWASM") B"
"$TARGET/debug/warp-runtime"
