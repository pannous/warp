#!/usr/bin/env bash
# scripts/build-slot.sh <log> <command…>: the command run in one of the machine's few build slots, its output appended
# to the log; its exit status. Every compile of the compiler goes through it (rebuild.sh, self-check.sh's stage two),
# shared by every session (user 2026-10-30: six rebuilds at once took the Mac): it runs in the first free slot of
# <store>/build-slots and waits for one when all are taken. WARP_BUILD_SLOTS names how many, WARP_COMPILER_STORE the store
# WARP_BUILD_RECORD=<dir>: the run measured into that folder (user 2026-10-31: every build profiled, all of it kept):
# time (/usr/bin/time -l: CPU, wall, peak memory) and the function profile of scripts/profile.sh -a (WARP_PROFILE=perfmap)
set -uo pipefail
DAY_BUILD_SLOTS=3  # user 2026-11-01: four warps at once, 3 builds plus 1 test run (warp test while building, core/native/tests.warp WHILE_BUILDING); 5 drove the load to ~460
LOCK_HELD_ELSEWHERE=75  # lockf's status when the slot is taken
FIRST_IN_LINE_RETRY=2  # seconds between the oldest waiter's tries of the slots
IN_LINE_RETRY=5  # seconds between the others' looks at the line
source "$(dirname "$0")/lib/platform.sh"
store=$warp_store
# <store>/build-slots/count, when present, sets the slots for every session at once (user 2026-11-02: at night as many
# as the load allows; the Supervisor raises and lowers it watching the load, notes/agents/night.md)
build_slots() { echo "${WARP_BUILD_SLOTS:-$(cat "$store/build-slots/count" 2>/dev/null || echo "$DAY_BUILD_SLOTS")}"; }
scripts=$(cd "$(dirname "$0")" && pwd)
log=${1:?usage: build-slot.sh <log> <command…>}
shift
record=${WARP_BUILD_RECORD:-}
command -v lockf >/dev/null || { "$@" >>"$log" 2>&1; exit; }
mkdir -p "$store/build-slots"
# A fair line (2026-11-01: thirteen waiters each retrying every slot let a newcomer pass an hour-old waiter): each
# waiter's ticket <seconds>.<pid> in the line, only the oldest live one tries the slots, and it leaves the line as soon
# as it holds one (inside the lock), so the next moves up while its build runs. A dead waiter's ticket is dropped
line=$store/build-slots/line
mkdir -p "$line"
ticket=$line/$(date +%s).$$
touch "$ticket"
trap 'rm -f "$ticket"' EXIT
tickets_in_order() { ls "$line" | sort -t. -k1,1n -k2,2n; }
drop_dead_tickets() {
	local name
	for name in $(ls "$line"); do kill -0 "${name##*.}" 2>/dev/null || rm -f "$line/$name"; done
}
place_in_line() { tickets_in_order | grep -nx "$(basename "$ticket")" | cut -d: -f1; }

# the measured command: under /usr/bin/time, the run profiled from its start until it ends
measured() {
	[ -n "$record" ] || { "$@"; return; }
	mkdir -p "$record"
	WARP_PROFILE=perfmap /usr/bin/time -lp -o "$record/time" "$@" &
	local timer=$! run=
	while [ -z "$run" ] && kill -0 $timer 2>/dev/null; do run=$(pgrep -P $timer | head -1) || sleep 0.05; done
	[ -n "$run" ] && "$scripts/profile.sh" -a "$run" -o "$record" &
	local profiler=$!
	wait $timer
	local status=$?
	wait $profiler 2>/dev/null
	return $status
}
export -f measured
export record scripts ticket

waited=
while :; do
	drop_dead_tickets
	[ -e "$ticket" ] || touch "$ticket"
	place=$(place_in_line)
	if [ "$place" = 1 ]; then
		for slot in $(seq "$(build_slots)"); do
			lockf -s -t 0 "$store/build-slots/$slot" bash -c 'rm -f "$ticket"; measured "$@"' measured "$@" >>"$log" 2>&1
			status=$?
			[ $status = $LOCK_HELD_ELSEWHERE ] || exit $status
		done
	fi
	[ -n "$waited" ] || { echo "build-slot.sh: all $(build_slots) build slots taken, waiting: number $place in line" >&2; waited=1; }
	if [ "$place" = 1 ]; then sleep $FIRST_IN_LINE_RETRY; else sleep $IN_LINE_RETRY; fi
done
