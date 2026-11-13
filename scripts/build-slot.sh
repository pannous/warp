#!/usr/bin/env bash
# scripts/build-slot.sh <log> <command…>: the command run in one of the machine's few build slots, its output appended
# to the log; its exit status. Every compile of the compiler goes through it (rebuild.sh, self-check.sh's stage two),
# shared by every session (six rebuilds at once took the Mac): it runs in the first free slot of
# <store>/build-slots and waits for one when all are taken. WARP_BUILD_SLOTS names how many, WARP_COMPILER_STORE the store
# WARP_BUILD_RECORD=<dir>: the run measured into that folder (every build profiled, all of it kept):
# time (/usr/bin/time -l: CPU, wall, peak memory) and the function profile of scripts/profile.sh -a (warp --profile),
# and steps: when each `rebuild step` began (stamped below)
set -uo pipefail
LOCK_HELD_ELSEWHERE=75  # lockf's status when the slot is taken
FIRST_IN_LINE_RETRY=2  # seconds between the oldest waiter's tries of the slots
IN_LINE_RETRY=5  # seconds between the others' looks at the line
source "$(dirname "$0")/lib/platform.sh"
store=$warp_store
scripts=$(cd "$(dirname "$0")" && pwd)
log=${1:?usage: build-slot.sh <log> <command…>}
shift
record=${WARP_BUILD_RECORD:-}
command -v lockf >/dev/null || { "$@" >>"$log" 2>&1; exit; }
mkdir -p "$store/build-slots"
# A fair line (thirteen waiters each retrying every slot let a newcomer pass an hour-old waiter): each
# waiter's ticket <seconds>.<pid> in the line, only the oldest live one tries the slots, and it leaves the line as soon
# as it holds one (inside the lock), so the next moves up while its build runs. A dead waiter's ticket is dropped
line=$store/build-slots/line
mkdir -p "$line"
# WARP_BUILD_FIRST=1 (after-push's build of main) goes to the head of the line: every branch merged with that main
# reuses its stored compiler instead of building it once more while it waits (one slot, 11 in line)
ticket=$line/$([ -n "${WARP_BUILD_FIRST:-}" ] && echo 0 || date +%s).$$
touch "$ticket"
trap 'rm -f "$ticket"' EXIT
tickets_in_order() { ls "$line" | sort -t. -k1,1n -k2,2n; }
drop_dead_tickets() {
	local name
	for name in $(ls "$line"); do kill -0 "${name##*.}" 2>/dev/null || rm -f "$line/$name"; done
}
place_in_line() { tickets_in_order | grep -nx "$(basename "$ticket")" | cut -d: -f1; }

# the measured command, a warp and its words: under /usr/bin/time, the run profiled (--profile) from its start until it ends
measured() {
	[ -n "$record" ] || { "$@"; return; }
	mkdir -p "$record"
	/usr/bin/time -lp -o "$record/time" "$1" --profile "${@:2}" &
	local timer=$! run=
	while [ -z "$run" ] && kill -0 $timer 2>/dev/null; do run=$(pgrep -P $timer | head -1) || sleep 0.05; done
	[ -n "$run" ] && "$scripts/profile.sh" -a "$run" -o "$record" &
	local profiler=$!
	wait $timer
	local status=$?
	wait $profiler 2>/dev/null
	return $status
}
# the command's output passed on unchanged, each `rebuild step <name>` line also noted in <record>/steps as "<name> <seconds
# since the run started>", and "end <seconds>" last (the vitals page's core, libraries and full build times)
stamped() {
	[ -n "$record" ] || { cat; return; }
	mkdir -p "$record"
	local line start=$SECONDS
	: >"$record/steps"
	while IFS= read -r line || [ -n "$line" ]; do
		printf '%s\n' "$line"
		case $line in "rebuild step "*) echo "${line#rebuild step } $((SECONDS - start))" >>"$record/steps" ;; esac
	done
	echo "end $((SECONDS - start))" >>"$record/steps"
}
export -f measured stamped
export record scripts ticket

waited=
while :; do
	drop_dead_tickets
	[ -e "$ticket" ] || touch "$ticket"
	place=$(place_in_line)
	if [ "$place" = 1 ]; then
		for slot in $(seq "$(build_slots)"); do
			lockf -s -t 0 "$store/build-slots/$slot" bash -c 'rm -f "$ticket"; set -o pipefail; measured "$@" 2>&1 | stamped' measured "$@" >>"$log" 2>&1
			status=$?
			[ $status = $LOCK_HELD_ELSEWHERE ] || exit $status
		done
	fi
	[ -n "$waited" ] || { echo "build-slot.sh: all $(build_slots) build slots taken, waiting: number $place in line" >&2; waited=1; }
	if [ "$place" = 1 ]; then sleep $FIRST_IN_LINE_RETRY; else sleep $IN_LINE_RETRY; fi
done
