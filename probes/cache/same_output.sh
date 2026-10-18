#!/bin/bash
# Every sample prints the same and exits the same with <warp binary> as with <reference binary>, and how many samples
# still emit twice ("emit rerun" phase, card takes-next). A sample whose reference output differs between two runs
# (random numbers, timings) is counted unstable, not different. Window samples (raylib, sdl) never run.
# usage: probes/cache/same_output.sh <warp binary> <reference binary> [sample filter]
set -u
warp=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
reference=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
filter=${3:-}
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$root/scratch/same_output
rm -rf "$work" && mkdir -p "$work" && cp -R "$root/samples" "$work/"
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_MODULE_CACHE=off WARP_CRASH_CARDS=0
run() { # <binary> <sample>: its output and exit status
	(cd "$(dirname "$2")" && timeout 30 "$1" "$2" </dev/null 2>&1; echo "exit $?")
}
without_phases() { grep -vE '^phase times \(|^ *[0-9.]+ ms  .* \([0-9]+×\)$'; }
same=0 different=0 unstable=0 reruns=0
while read -r sample; do
	output=$(WARP_PHASE_TIMES=1 run "$warp" "$sample")
	if echo "$output" | grep -q " emit rerun ("; then
		reruns=$((reruns + 1))
		echo "RERUN ${sample#$work/}"
	fi
	expected=$(run "$reference" "$sample")
	if [ "$(echo "$output" | without_phases)" == "$expected" ]; then
		same=$((same + 1))
	elif [ "$(run "$reference" "$sample")" != "$expected" ]; then
		unstable=$((unstable + 1))
	else
		different=$((different + 1))
		echo "DIFFERENT ${sample#$work/}"
	fi
done < <(find "$work/samples" -name "*.warp" | grep -- "$filter" | grep -vE "raylib|sdl" | sort)
echo "same: $same, different: $different, unstable: $unstable, emitted twice: $reruns"
