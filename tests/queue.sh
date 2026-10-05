#!/bin/bash
# The one gate for running warp tests on this machine (notes/roles.md, "Enforcement"): every test run waits for one
# machine-wide lock, so at most one test binary runs at a time; the others queue instead of overloading the Mac.
#   tests/queue.sh [cargo test args]      e.g. tests/queue.sh -- test_traits      (cargo --offline test …)
#   tests/queue.sh cargo browser-test x   any cargo command
#   tests/queue.sh ./test.sh              a script (test.sh re-enters through here by itself)
# A run that already holds the lock (WARP_TEST_LOCKED) runs directly, so nesting never deadlocks.
# Waiters are served first come, first served by ticket files in $QUEUE; a full ./test.sh (the Integrator's run)
# gets priority and takes the next slot ahead of targeted runs. Tickets of dead processes are dropped.
# Every run is niced so the user's apps stay responsive; targeted runs use at most TARGETED_TEST_THREADS threads
# (RUST_TEST_THREADS, which test.sh's explicit --test-threads overrides for the full suite).
LOCK="${WARP_TEST_LOCK:-$HOME/.cargo/warp-tests.lock}"
QUEUE="$LOCK.queue"
POLL_SECONDS=2
NICENESS=10
TARGETED_TEST_THREADS=4

if [ "$1" = "cargo" ] || [ -x "$1" ]; then run=("$@"); else run=(cargo --offline test "$@"); fi
[ -n "$WARP_TEST_LOCKED" ] && exec "${run[@]}"

# lockf is the Mac's (BSD) lock tool, Linux (cloud VMs) has flock; with neither there is no lock to take
if command -v lockf >/dev/null; then
	lock_is_free=(lockf -s -t 0 "$LOCK" true); hold_lock=(lockf -k "$LOCK")
elif command -v flock >/dev/null; then
	lock_is_free=(flock -n "$LOCK" true); hold_lock=(flock "$LOCK")
else
	echo "test queue: neither lockf nor flock found, running WITHOUT the warp test lock" >&2
	WARP_TEST_LOCKED=1 exec "${run[@]}"
fi
mkdir -p "$(dirname "$LOCK")"

case "${run[0]}" in *test.sh) priority=0 ;; *) priority=1; export RUST_TEST_THREADS="${RUST_TEST_THREADS:-$TARGETED_TEST_THREADS}" ;; esac
mkdir -p "$QUEUE"
ticket="$QUEUE/$priority-$(date +%s)-$$"
echo "$PWD ${run[*]}" > "$ticket"

drop_stale_tickets() {
	for t in "$QUEUE"/*; do
		[ -e "$t" ] && ! kill -0 "${t##*-}" 2>/dev/null && rm -f "$t"
	done
}
first_ticket() { ls "$QUEUE" | sort | head -1; }

drop_stale_tickets
if [ "$(first_ticket)" != "${ticket##*/}" ] || ! "${lock_is_free[@]}"; then
	echo "test queue: waiting for the warp test lock ($(cat "$LOCK.owner" 2>/dev/null || echo unknown holder)," \
		"$(($(ls "$QUEUE" | sort | grep -n "^${ticket##*/}$" | cut -d: -f1) - 1)) ahead of us)" >&2
fi
while drop_stale_tickets; [ "$(first_ticket)" != "${ticket##*/}" ]; do sleep $POLL_SECONDS; done

# exec keeps our PID, so the ticket stays valid while we wait for the lock and run; it is removed when the run ends
export WARP_TEST_LOCKED=1
exec "${hold_lock[@]}" bash -c 'trap "rm -f $1" EXIT; echo "$PPID $PWD ${*:2}" > "$0.owner"; "${@:2}"' \
	"$LOCK" "$ticket" nice -n "$NICENESS" "${run[@]}"
