#!/bin/sh
# The probe's own lines of a run log (gh run view --log > file): job | message, the steps' setup lines dropped
grep -E "WebGPU on the page|shader examples" "$1" | awk -F'\t' '{ sub(/^.{29}/, "", $3); if ($3 !~ /^(##|shell:|env:|  FIREFOX|\x1b|$)/) print substr($1, 1, 60) " | " $3 }'
