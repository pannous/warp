#!/bin/bash
# the programs of a list (one path a line) through emitter.warp (WARP_SELF_HOSTED=emit) with two libs, before and after
# a change of core: prints the programs whose output differs (card emitter-split)
#   probes/emitter_split_check.sh <programs.txt> <lib before> <lib after> <out dir>
programs=$1 before=$2 after=$3 out=$4
mkdir -p "$out/before" "$out/after"
run() { # lib, output file, program
	WARP_LIB="$1" WARP_SELF_HOSTED=emit WARP_NO_WINDOW=1 WARP_HINTS=0 timeout 120 /Users/me/dev/bin/warp "$3" > "$2" 2>&1 < /dev/null
}
while read -r program; do
	name=$(echo "$program" | tr '/' '_')
	run "$before" "$out/before/$name" "$program" &
	run "$after" "$out/after/$name" "$program"
	wait
	cmp -s "$out/before/$name" "$out/after/$name" || echo "differs: $program"
done < "$programs"
echo "checked $(wc -l < "$programs") programs"
