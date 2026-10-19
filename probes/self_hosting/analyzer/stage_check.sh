#!/bin/bash
# Runs each argument (warp code) with the self-hosted analysis check (WARP_SELF_ANALYSIS, src/compiler/self_hosted.rs
# check_analysis) using the checkout's own warp, printing only the result line or the disagreement.
# Usage: probes/self_hosting/analyzer/stage_check.sh 'code' ['code'…]   (from the repo root)
for code in "$@"; do
	result=$(WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_SELF_HOSTED=1 WARP_SELF_ANALYSIS=1 scratch/warp "$code" 2>&1 | grep -E "disagrees|failed|^»" | head -1)
	echo "$code  ⇒  $result"
done
