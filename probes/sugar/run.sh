#!/bin/bash
# runs each probe in data/sugar/cases.txt (id<TAB>expected<TAB>code, \n escapes) through the warp binary
BIN=${BIN:-/opt/cargo/warp-sugar/debug/warp}
cd "$(dirname "$0")/../../data/sugar"
while IFS=$'\t' read -r id expected code; do
  [[ -n "$1" && "$id" != $1* ]] && continue
  got=$(timeout 20 "$BIN" eval "$(printf "$code")" 2>&1 | tail -1)
  printf '%-4s want %-10s got %s\n' "$id" "$expected" "$got"
done < cases.txt
