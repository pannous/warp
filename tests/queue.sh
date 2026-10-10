#!/bin/bash
# The one gate for running warp tests on this machine (notes/roles.md, "Enforcement"): every test run waits for one
# machine-wide lock, so at most one test binary runs at a time; the others queue instead of overloading the Mac.
#   tests/queue.sh [cargo test args]      e.g. tests/queue.sh -- test_traits      (cargo --offline test …)
#   tests/queue.sh cargo browser-test x   any cargo command
#   tests/queue.sh ./test.sh              a script (test.sh re-enters through here by itself)
# A run that already holds the lock (WARP_TEST_LOCKED) runs directly, so nesting never deadlocks.
# Waiters are served first come, first served by ticket files in $QUEUE; a full ./test.sh (the Integrator's run)
# gets priority and takes the next slot ahead of targeted runs. Tickets of dead processes are dropped.
# Targeted runs are niced so the user's apps stay responsive and use at most TARGETED_TEST_THREADS threads
# (RUST_TEST_THREADS); the full suite keeps normal priority, because its task-overlap tests measure wall clock.
LOCK="${WARP_TEST_LOCK:-$HOME/.cargo/warp-tests.lock}"
QUEUE="$LOCK.queue"
POLL_SECONDS=2
TARGETED_NICENESS=10
TARGETED_TEST_THREADS=4
# A run with a process using more memory than this is killed: on 2026-10-09 one test grew to 154 GB, swap ran out and
# macOS took down the Claude app with every agent session. Per process, because parallel rustc jobs add up legitimately;
# top's MEM counts compressed memory, ps's RSS doesn't.
MEMORY_CAP_GB="${WARP_TEST_MEMORY_CAP_GB:-4}"
MEMORY_POLL_SECONDS=1
# A test binary running longer than this is killed, naming it and the program it waits for: batch 137 hung 294 s on
# finger_paint.warp (card tests-queue). The tests binary takes ~150 s of a full run, 266 s at most so far.
TEST_TIME_LIMIT_SECONDS="${WARP_TEST_TIME_LIMIT:-600}"
TEST_BINARY_PATTERN='^[^ ]*/deps/[^/ ]+-[0-9a-f]{16}( |$)' # the executable of cargo's test runs: …/deps/<crate>-<hash>
# panics provoked by tests file no to-do board cards (src/crash_card.rs); test.sh runs through here too
export WARP_CRASH_CARDS=0

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

case "${run[0]}" in
	*test.sh) priority=0; niceness=0 ;;
	*) priority=1; niceness=$TARGETED_NICENESS; export RUST_TEST_THREADS="${RUST_TEST_THREADS:-$TARGETED_TEST_THREADS}" ;;
esac
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

descendants() {
	local child
	for child in $(pgrep -P "$1"); do echo "$child"; descendants "$child"; done
}
# memory in MB of the largest of the given PIDs, from top's MEM column (e.g. 1872K, 26M, 3G); top -pid refuses a PID
# that has exited meanwhile, so it lists all processes and awk picks ours
largest_memory_mb() {
	top -l 1 -s 0 -stats pid,mem | awk -v pids=" $* " '
		$1 ~ /^[0-9]+$/ && index(pids, " " $1 " ") { value = $2; sub(/[+-]$/, "", value); unit = substr(value, length(value)); amount = value + 0
			mb = unit == "G" ? amount * 1024 : unit == "M" ? amount : unit == "K" ? amount / 1024 : amount / 1048576; if (mb > largest) largest = mb }
		END { printf "%d", largest }'
}
# "pid seconds command" of the given PIDs, oldest first; ps shows the elapsed time as [[dd-]hh:]mm:ss
running_times() {
	ps -o pid=,etime=,command= -p "$(IFS=,; echo "$*")" 2>/dev/null | awk '{
		split($2, part, /[-:]/); n = length(part); seconds = part[n] + 60 * part[n - 1]
		if (n > 2) seconds += 3600 * part[n - 2]; if (n > 3) seconds += 86400 * part[n - 3]
		$2 = seconds; print }' | sort -k2 -rn
}
# a test binary of the run over TEST_TIME_LIMIT_SECONDS is killed with what it started; the run fails, naming both
kill_overdue_test_binaries() {
	local pid seconds command children
	running_times "$@" | while read -r pid seconds command; do
		[ "$seconds" -gt "$TEST_TIME_LIMIT_SECONDS" ] && echo "$command" | grep -qE "$TEST_BINARY_PATTERN" || continue
		children=($(descendants "$pid"))
		echo "test queue: KILLED the test binary after ${seconds} s, over the time limit of ${TEST_TIME_LIMIT_SECONDS} s" \
			"(WARP_TEST_TIME_LIMIT): $command" >&2
		[ ${#children[@]} -gt 0 ] && echo "test queue: it was waiting for: $(running_times "${children[@]}" | head -1 | cut -d' ' -f3-)" >&2
		kill -9 "$pid" "${children[@]}" 2>/dev/null
	done
}
# runs beside the test run (our PID survives the exec below): kills its whole process tree above the memory cap
# (Mac only: top's MEM) and a test binary over the time limit
watch_run() {
	local run_pid=$1 cap_mb=$((MEMORY_CAP_GB * 1024)) pids used_mb
	while kill -0 "$run_pid" 2>/dev/null; do
		sleep $MEMORY_POLL_SECONDS
		pids=($(descendants "$run_pid"))
		[ ${#pids[@]} -eq 0 ] && continue
		kill_overdue_test_binaries "${pids[@]}"
		[ "$(uname)" = Darwin ] || continue
		used_mb=$(largest_memory_mb "${pids[@]}")
		if [ "$used_mb" -gt "$cap_mb" ]; then
			echo "test queue: KILLED the run, a process used ${used_mb} MB, over the memory cap of ${MEMORY_CAP_GB} GB" \
				"(WARP_TEST_MEMORY_CAP_GB): ${run[*]}" >&2
			kill -9 "${pids[@]}" 2>/dev/null
			return
		fi
	done
}
watch_run $$ &

# exec keeps our PID, so the ticket stays valid while we wait for the lock and run; it is removed when the run ends
export WARP_TEST_LOCKED=1
export WARP_NO_WINDOW=1 # a painting program writes PNGs, never a window during tests (src/paint.rs)
exec "${hold_lock[@]}" bash -c 'trap "rm -f $1" EXIT; echo "$PPID $PWD ${*:2}" > "$0.owner"; "${@:2}"' \
	"$LOCK" "$ticket" nice -n "$niceness" "${run[@]}"
