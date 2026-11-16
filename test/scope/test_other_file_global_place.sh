#!/usr/bin/env bash
# other_file_global names the assignment in the used file, not the program's `global angle` line and not
# the function's first parameter (fish.warp:1:11 was jaw_point's `point` under the program's file).
# decision loop-locals; a used file's nodes name its own lines (positions.warp source_files).
# Run: bash test/scope/test_other_file_global_place.sh
# WARP defaults to the warp on PATH; a tree's: WARP=<tree>/data/gate/warp WARP_LIB=<tree>/lib
set -e
cd "$(dirname "$0")/../.."
warp=${WARP:-warp}
export WARP_WINDOW=0
lib=()
if [ -n "${WARP_LIB:-}" ]; then lib=(--lib "$WARP_LIB"); fi

said=$("$warp" --no-window "${lib[@]}" test/fixtures/scope_span_program.warp 2>&1 | tail -1)
# scope_span_jaw.warp line 7, column 2: `\tangle = 1` (a used file, lines already of the file)
case $said in
	*"test/fixtures/scope_span_jaw.warp:7:2 "*) echo "✓ other_file_global names the used-file assignment: $said" ;;
	*) echo "✗ expected test/fixtures/scope_span_jaw.warp:7:2 as the place, got: $said"; exit 1 ;;
esac

