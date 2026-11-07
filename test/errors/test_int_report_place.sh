#!/usr/bin/env bash
# A value that does not fit a declared variable is reported where it is assigned, not at the declaration:
# `x: int = 2; x = 2.5` names 1:13 (card int-reports). Run it with bash test/errors/test_int_report_place.sh; the warp
# under test is WARP (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
said=$("$warp" 'x: int = 2; x = 2.5' 2>&1 | tail -1)
case $said in
	*"at 1:13"*) echo "✓ the failing assignment names its own place: $said" ;;
	*) echo "✗ expected the error at 1:13, got: $said"; exit 1 ;;
esac
