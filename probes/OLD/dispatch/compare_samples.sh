#!/bin/bash
# Run every sample with two warp binaries and print the samples whose last line differs (card one-dispatch):
#   probes/dispatch/compare_samples.sh <old warp> <new warp> [sample folder]
OLD=$1 NEW=$2 FOLDER=${3:-samples}
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_CRASH_CARDS=0
last_line() { timeout 20 "$1" "$2" 2>&1 < /dev/null | grep -v '^\s*$' | tail -1 | sed -E 's/[0-9]+\.wav/N.wav/'; }
find "$FOLDER" -name '*.warp' | sort | while read -r sample; do
	before=$(last_line "$OLD" "$sample")
	after=$(last_line "$NEW" "$sample")
	[ "$before" != "$after" ] && printf '%s\n  before: %s\n  after:  %s\n' "$sample" "$before" "$after"
done
