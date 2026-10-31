#!/usr/bin/env bash
# scripts/build-slot.sh <log> <command…>: the command run in one of the machine's few build slots, its output appended
# to the log; its exit status. Every compile of the compiler goes through it (rebuild.sh, self-check.sh's stage two),
# shared by every session (user 2026-10-30: six rebuilds at once took the Mac): it runs in the first free slot of
# <store>/build-slots and waits for one when all are taken. WARP_BUILD_SLOTS names how many, WARP_COMPILER_STORE the store
# WARP_BUILD_RECORD=<dir>: the run measured into that folder (user 2026-10-31: every build profiled, all of it kept):
# time (/usr/bin/time -l: CPU, wall, peak memory) and the function profile of scripts/profile.sh -a (WARP_PROFILE=perfmap)
set -uo pipefail
BUILD_SLOTS=${WARP_BUILD_SLOTS:-5}
LOCK_HELD_ELSEWHERE=75  # lockf's status when the slot is taken
if [ "$(uname)" = Darwin ]; then store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}; else store=${WARP_COMPILER_STORE:-$HOME/.cache/warp/compilers}; fi
scripts=$(cd "$(dirname "$0")" && pwd)
log=${1:?usage: build-slot.sh <log> <command…>}
shift
record=${WARP_BUILD_RECORD:-}
command -v lockf >/dev/null || { "$@" >>"$log" 2>&1; exit; }
mkdir -p "$store/build-slots"

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
export record scripts

waited=
while :; do
	for slot in $(seq "$BUILD_SLOTS"); do
		lockf -s -t 0 "$store/build-slots/$slot" bash -c 'measured "$@"' measured "$@" >>"$log" 2>&1
		status=$?
		[ $status = $LOCK_HELD_ELSEWHERE ] || exit $status
	done
	[ -n "$waited" ] || { echo "build-slot.sh: all $BUILD_SLOTS build slots taken, waiting" >&2; waited=1; }
	sleep 10
done
