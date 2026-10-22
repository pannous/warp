#!/bin/bash
# scripts/cloud-test.sh: a pooled session whose job's result is on origin is free (card cloud-lane-full: the job name's
# launch time was taken for part of the run name, so finished sessions stayed busy and were dropped as "never answered").
# The answer of origin is stubbed: no results branch outlives its collection
set -u
eval "$(sed '$d' "$(dirname "$0")/../scripts/cloud-test.sh")"
git() { [ "$1 $2 $3 $4" = "-C $MAIN_CHECKOUT ls-remote --exit-code" ] && { echo "asked for $6" >&2; [ "$6" = "refs/heads/$RESULTS_PREFIX/main-0a5f10e7-a4fffa0dfd73" ]; return; }; command git "$@"; }
file=$(cd "$(dirname "$0")/.." && pwd)/scratch/pool_free_session
mark_session "$file" main-0a5f10e7-a4fffa0dfd73-1792636329-24501 1
session_is_free "$file" && echo "free (expected)" || echo "BUSY: the result was not seen"
