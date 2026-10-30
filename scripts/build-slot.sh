#!/usr/bin/env bash
# scripts/build-slot.sh <log> <command…>: the command run in one of the machine's few build slots, its output appended
# to the log; its exit status. Every compile of the compiler goes through it (rebuild.sh, self-check.sh's stage two),
# shared by every session (user 2026-10-30: six rebuilds at once took the Mac): it runs in the first free slot of
# <store>/build-slots and waits for one when all are taken. WARP_BUILD_SLOTS names how many, WARP_COMPILER_STORE the store
set -uo pipefail
BUILD_SLOTS=${WARP_BUILD_SLOTS:-5}
LOCK_HELD_ELSEWHERE=75  # lockf's status when the slot is taken
if [ "$(uname)" = Darwin ]; then store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}; else store=${WARP_COMPILER_STORE:-$HOME/.cache/warp/compilers}; fi
log=${1:?usage: build-slot.sh <log> <command…>}
shift
command -v lockf >/dev/null || { "$@" >>"$log" 2>&1; exit; }
mkdir -p "$store/build-slots"
waited=
while :; do
	for slot in $(seq "$BUILD_SLOTS"); do
		lockf -s -t 0 "$store/build-slots/$slot" "$@" >>"$log" 2>&1
		status=$?
		[ $status = $LOCK_HELD_ELSEWHERE ] || exit $status
	done
	[ -n "$waited" ] || { echo "build-slot.sh: all $BUILD_SLOTS build slots taken, waiting" >&2; waited=1; }
	sleep 10
done
