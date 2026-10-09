#!/bin/zsh
# Coverage of W0 (the Lean type model, notes/type_theory.md): which sampled is!() programs the exporter takes in.
# usage: probes/type_coverage.sh <warp binary> <programs file, one per line> > data/types/coverage.txt
# Each line: IN (exported; the model's and warp's verdicts follow) or the exporter's "not in W0: …" reason.
warp=$1
programs=$2
while IFS= read -r program; do
	report=$("$warp" types "$program" 2>&1)
	# hints and warnings may come before the W0 line
	if echo $report | grep -q '^W0: '; then echo "IN $(echo $report | grep -E '^(model|warp):' | tr '\n' ' ')"
	else echo $report | grep -m1 "not in W0" || echo "OTHER $(echo $report | grep -v '^\(hint\|warning\|note\|      \)' | head -1)"
	fi
done < "$programs"
