#!/bin/bash
# warp format on a copy of samples/ and lib/: every formatted file must lower to the same program as before, and
# formatting again must change nothing (card warp-format). Usage: probes/warp_format_samples.sh [warp binary]
REPO=$(cd "$(dirname "$0")/.." && pwd)
WARP=${1:-$REPO/scratch/warp}
WORK=$REPO/scratch/warp_format_samples
export WARP_NO_WINDOW=1 WARP_HINTS=0

# warp reads a piped stdin as its program: < /dev/null keeps it off the loop's file list
lowered() { timeout 20 "$WARP" lower "$(cat "$1")" < /dev/null 2>&1 | sed 's/[0-9]*:[0-9]*//g'; }

rm -rf "$WORK" && mkdir -p "$WORK" && cp -R "$REPO/samples" "$REPO/lib" "$WORK/"
"$WARP" format "$WORK" | tail -1
compared=0 different=0
while read -r formatted; do
	original=$REPO/${formatted#"$WORK"/}
	compared=$((compared + 1))
	[[ "$(lowered "$original")" == "$(lowered "$formatted")" ]] || { echo "❌ lowers differently: $original"; different=$((different + 1)); }
done < <({ diff -rq "$REPO/samples" "$WORK/samples"; diff -rq "$REPO/lib" "$WORK/lib"; } 2>/dev/null | sed -n 's/^Files .* and \(.*\) differ$/\1/p')
again=$("$WARP" format check "$WORK" | tail -1)
echo "$compared formatted files compared, $different lower differently; again: $again"
rm -rf "$WORK"
[[ $different == 0 && $again == *" 0 to format"* ]]
