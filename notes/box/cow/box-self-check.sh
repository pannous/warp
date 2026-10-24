#!/bin/bash
# box-self-check.sh <tree> [warp binary]: warp rebuild pure + pure hello, warp rebuild self + self hello
# A tree with scripts/box-self-check.sh gets that check (the store's pair, the shared binary, no rebuild; card
# scripts-off-cargo); an older tree this one, with the shared binary by default instead of its own target/debug/warp
[ -x "$1/scripts/box-self-check.sh" ] && { [ -n "${2-}" ] && export WARP_BINARY=$2; exec "$1/scripts/box-self-check.sh" "$1"; }
t=$1; w=${2:-/workspace/cow/sol-warp-latest}; cd $t || exit 1
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$t/lib
for lane in pure self; do
  s=$(date +%s)
  out=$($w rebuild $lane 2>&1) || { echo "SELF CHECK RED: rebuild $lane failed: $(echo "$out" | grep -v '^warning\|^ *[0-9]* |\|^ *|' | tail -5)"; exit 1; }
  said=$($w $lane samples/hello/hello.warp 2>&1 | tail -1)
  echo "$lane: rebuild $(( $(date +%s)-s ))s, hello said: $said"
  [[ $said == *Hello* ]] || { echo "SELF CHECK RED: $lane hello"; exit 1; }
done
echo "SELF CHECK ok"
