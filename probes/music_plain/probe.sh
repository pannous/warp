#!/bin/bash
# Each argument a warp program, evaluated by this checkout's scratch/warp without window or sound
cd "$(dirname "$0")/../.."
for program in "$@"; do
	echo "== $program"
	WARP_NO_WINDOW=1 WARP_HINTS=0 timeout 60 scratch/warp eval "$program" 2>&1 | tail -12
done
