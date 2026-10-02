#!/bin/sh
# runs every listparams probe; each prints 4, or 'b' for the indexed text parameters (20-23)
WARP="${CARGO_TARGET_DIR:-$HOME/.cargo/shared-target}/debug/warp"
for probe in "$(dirname "$0")"/*.wasp; do
  printf '%s  %s  => ' "$(basename "$probe")" "$(cat "$probe")"
  "$WARP" "$probe" 2>&1 | tail -1
done
