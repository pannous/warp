#!/bin/bash
# Every sample compiles to the same wasm with the lowered-modules cache cold, warm, and off (card prelude-cache-runs).
# usage: probes/cache/cold_warm_equal.sh <warp binary> [sample filter]
# Works on a copy of samples/ in scratch/ (compile --wasm writes <file>.wasm beside each file); prints each difference
set -u
warp=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
filter=${2:-}
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/scratch/cold_warm
rm -rf "$work" && mkdir -p "$work/cache" && cp -R "$root/samples" "$work/"
export WARP_NO_WINDOW=1 WARP_HINTS=0
compile() { # <cache setting> <sample> <copy name>
	WARP_MODULE_CACHE=$1 timeout 120 "$warp" compile --wasm "$2" >/dev/null 2>&1
	if [ -f "${2%.warp}.wasm" ]; then mv "${2%.warp}.wasm" "$3"; else echo none > "$3"; fi
}
same=0 different=0
while read -r sample; do
	compile off "$sample" "$work/off.wasm"
	compile "$work/cache" "$sample" "$work/cold.wasm"
	compile "$work/cache" "$sample" "$work/warm.wasm"
	if cmp -s "$work/cold.wasm" "$work/warm.wasm" && cmp -s "$work/off.wasm" "$work/cold.wasm"; then
		same=$((same + 1))
	else
		different=$((different + 1))
		echo "DIFFERENT ${sample#$work/} ($(md5 -q "$work/off.wasm" | cut -c1-8) $(md5 -q "$work/cold.wasm" | cut -c1-8) $(md5 -q "$work/warm.wasm" | cut -c1-8)): off/cold $(cmp -s "$work/off.wasm" "$work/cold.wasm" && echo same || echo differ), cold/warm $(cmp -s "$work/cold.wasm" "$work/warm.wasm" && echo same || echo differ)"
	fi
done < <(find "$work/samples" -name "*.warp" | grep -- "$filter" | sort)
echo "same: $same, different: $different, lowered modules kept: $(ls "$work/cache" | grep -c '\.lowered$')"
