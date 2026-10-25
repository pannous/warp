#!/bin/bash
wt=/workspace/wt-parser2
mkdir -p /workspace/selfcheck-lock
warp=/workspace/selfcheck-lock/warp
cp -L /workspace/cow/warp-rust-latest $warp || exit 1
export PATH=/workspace/tools/binaryen-version_133/bin:$PATH
cd $wt || exit 1
# a tree with scripts/box-self-check.sh: the store's pair and the shared binary, no rebuild (card scripts-off-cargo)
[ -x $wt/scripts/box-self-check.sh ] && { WARP_BINARY=$warp exec $wt/scripts/box-self-check.sh $wt; }
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$wt/lib
for lane in pure self; do
  t0=$(date +%s)
  built=$($warp rebuild $lane 2>&1); rc=$?
  t1=$(date +%s)
  echo "== rebuild $lane rc=$rc $((t1-t0))s"; echo "$built" | grep -v '^warning\|^ *[0-9]* |\|^ *|' | tail -5
  [ $rc -ne 0 ] && { echo "SELF CHECK RED: warp rebuild $lane failed"; exit 1; }
  out=$($warp $lane samples/hello/hello.warp 2>&1); t2=$(date +%s)
  said=$(echo "$out" | tail -1)
  echo "== $lane hello $((t2-t1))s, last lines:"; echo "$out" | tail -3
  [[ $said == *Hello* ]] && echo "SELF CHECK ok ($lane): $said" || { echo "SELF CHECK RED ($lane): warp $lane hello said: $said"; exit 1; }
done
