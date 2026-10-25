#!/bin/bash
while pgrep -f 'CARGO_TARGET_DIR=/workspace/target-main cargo build' >/dev/null; do sleep 15; done
tail -1 /workspace/build-main.log | grep -q 'EXIT 0' || { echo "BUILD FAILED"; tail -20 /workspace/build-main.log; exit 1; }
h=$(git -C /workspace/wt-main rev-parse --short=9 HEAD)
dst=/workspace/cow/warp-rust-$h
cp /workspace/target-main/release/warp $dst && chmod +x $dst
echo "INSTALLED $dst"
cd /workspace/wt-main
export PATH=/workspace/tools/binaryen-version_133/bin:$PATH WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=/workspace/wt-main/lib
t0=$(date +%s)
out=$(flock /tmp/warp-box-heavy.lock $dst rebuild pure 2>&1); rc=$?
echo "rebuild pure rc=$rc $(( $(date +%s)-t0 ))s"; echo "$out" | grep -v '^warning' | tail -5
git status --short | head
[ $rc -eq 0 ] || echo "SMOKE FAILED: rebuild pure rc=$rc"
exit $rc
