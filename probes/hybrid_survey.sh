#!/bin/zsh
# What still falls back from the pure lane to the WARP_SELF_HOSTED=1 hybrid lane (card retire-hybrid-lane): runs every
# STEP-th code line of the test pairs through `warp` with WARP_SELF_HOSTED=pure and counts the pure lane's refusals by
# reason. Heavy: run it inside scripts/slot.sh.
# usage: probes/hybrid_survey.sh [<lib>, default ./lib] [<step>, default 4]   → data/hybrid/survey-<date>.txt
lib=${1:-$PWD/lib}
step=${2:-4}
out=data/hybrid/survey-$(date +%Y%m%d-%H%M).txt
refusals=data/hybrid/refusals.tmp
mkdir -p data/hybrid
: > $refusals
programs=0
index=0
for file in tests/*/*.pairs.warp; do
  grep -v '^#\|^$' $file | while IFS=$'\t' read -r code expected rest; do
    index=$((index + 1))
    (( index % step == 0 )) || continue
    code=${code#\'}; code=${code%\'}; code=${code#«}; code=${code%»}
    programs=$((programs + 1))
    WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_SELF_HOSTED=pure WARP_EMIT_REFUSALS=$refusals WARP_LIB=$lib timeout 60 /Users/me/dev/bin/warp "$code" > /dev/null 2>&1
  done
done
ran=$(grep -c '^pure: ran' $refusals)
{
  echo "$programs programs sampled (every ${step}th pair line), $ran ran pure"
  echo "refusals by reason (digits as #):"
  grep -v '^pure: ran' $refusals | sed -E 's/[0-9]+/#/g; s/`[^`]*`/`…`/g' | sort | uniq -c | sort -rn
} > $out
rm $refusals
echo $out
