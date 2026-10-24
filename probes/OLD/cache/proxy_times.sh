#!/bin/bash
# The success measure of card prelude-cache-runs: a program calling a warp-written prelude word against one using a Rust
# builtin, each phase's minimum over runs (fresh literals, so the program cache misses)
# usage: probes/cache/proxy_times.sh <warp binary> [runs] [phase pattern]
set -u
warp=$1 runs=${2:-10} phases=${3:-lowering, all|meaning passes|program pass 2.4|program source passes}
export WARP_NO_WINDOW=1 WARP_PHASE_TIMES=1
for program in 'print [1 2 N]' 'print to_base(N, 2)'; do
	for run in $(seq "$runs"); do
		"$warp" "${program//N/$((RANDOM + 100))}" 2>&1 >/dev/null | grep -E "  ($phases) \("
	done | awk -v program="$program" '{ name = $0; sub(/^ *[0-9.]+ ms  /, "", name); sub(/ \(.*/, "", name)
		if (!(name in least) || $1 < least[name]) least[name] = $1 }
		END { printf "%-22s", program; for (name in least) printf "  %s %.1f", name, least[name]; print "" }'
done
