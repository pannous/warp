#!/bin/bash
# Smoke: `warp build --exe` = prebuilt warp-runtime stub + appended .cwasm (notes/aot.md §Next / g-qV8Y).
# usage: probes/aot/build_exe.sh [warp binary]     (run from the repo root)
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
TARGET=$(cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
cargo build --offline --bin warp 2>&1 | tail -2
cargo build --offline -p warp-runtime 2>&1 | tail -2
WARP=${1:-$TARGET/debug/warp}
STUB=$TARGET/debug/warp-runtime
mkdir -p scratch/aot
cp -f samples/ackermann.wasp scratch/aot/ackermann.wasp

# Positive: standalone exe prints ackermann(3,3)=61 with no Rust toolchain for the user
WARP_RUNTIME_STUB=$STUB "$WARP" build --exe scratch/aot/ackermann.wasp
EXE=scratch/aot/ackermann.exe
test -x "$EXE"
OUT=$("$EXE")
echo "standalone said: $OUT"
test "$OUT" = "61"

# Negative: a program the stub cannot host (run_block) is refused at build time, naming the import
printf '%s\n' 'y = data 6*7; interpret y' > scratch/aot/needs_run_block.wasp
set +e
NEG=$(WARP_RUNTIME_STUB=$STUB "$WARP" build --exe scratch/aot/needs_run_block.wasp 2>&1)
NEG_EXIT=$?
set -e
echo "$NEG" | tail -1
test "$NEG_EXIT" -ne 0
echo "$NEG" | grep -q 'run_block'
echo "ok build_exe"
