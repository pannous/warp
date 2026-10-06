#!/bin/bash
# Probe function-call features ported from other languages: each line "code ||| expected"
warp="$(dirname "$0")/../scratch/warp"
cases="${1:-$(dirname "$0")/function_calls.md}"
while IFS= read -r line; do
	[[ -z "$line" || "$line" == \#* ]] && continue
	code="${line%% ||| *}"; want="${line##* ||| }"
	got="$(timeout 10 "$warp" --no-ask eval "$code" 2>&1 | tail -1 | sed -e "s/^» //" -e "s/^\"\(.*\)\"$/\1/")"
	if [ "$got" == "$want" ]; then echo "ok   $code"; else echo "FAIL $code  => $got  (want $want)"; fi
done < "$cases"
