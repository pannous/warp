#!/usr/bin/env bash
# scripts/job-slot.sh <command…>: the command run in one of the machine's job slots, niced, its output and exit status
# its own. Every wap test script and census run goes through it, from every worktree (test/wap/run.sh,
# ratchet_census.sh, vitals.py): it takes the first free slot of <store>/job-slots, job_slots() of them
# (scripts/lib/platform.sh), and waits for one when all are taken, saying so once
set -uo pipefail
RETRY_SECONDS=1
# the integrator's landing trees, warp's and wap's: their runs may take one more slot, which no other run takes, so a
# landing never starves (a tree whose name only starts the same, integrate2, is no landing tree)
INTEGRATION_TREES=(/Volumes/Worktrees/integrate /Volumes/Worktrees/wap-land)
source "$(dirname "$0")/lib/platform.sh"
source "$(dirname "$0")/lib/slots.sh"
[ $# -gt 0 ] || { echo "usage: job-slot.sh <command…>" >&2; exit 2; }
command -v lockf >/dev/null || exec nice -n 19 "$@"
slots=$(job_slots)
for tree in "${INTEGRATION_TREES[@]}"; do [[ " $PWD/ $*/" == *" $tree/"* ]] && { slots=$((slots + 1)); break; }; done
waited=
while :; do
	slot_tried "$warp_store/job-slots" "$slots" nice -n 19 "$@"
	status=$?
	[ $slot_held = no ] || exit $status
	[ -n "$waited" ] || { echo "job-slot.sh: all $(job_slots) job slots taken, waiting" >&2; waited=1; }
	sleep $RETRY_SECONDS
done
