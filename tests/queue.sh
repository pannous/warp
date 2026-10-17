#!/bin/bash
# The one gate for running warp tests on this machine (notes/roles.md, "Enforcement"): every test run waits for one of
# TARGETED_SLOTS machine-wide slot locks, so at most that many targeted runs go at once and a full ./test.sh runs alone
# (it holds every slot); the others queue instead of overloading the Mac.
#   tests/queue.sh [cargo test args]      e.g. tests/queue.sh -- test_traits      (cargo --offline test …)
#   tests/queue.sh cargo browser-test x   any cargo command
#   tests/queue.sh ./test.sh              a script (test.sh re-enters through here by itself)
# A run that already holds a slot (WARP_TEST_LOCKED) runs directly, so nesting never deadlocks.
# Waiters are served first come, first served by ticket files in $QUEUE: only the first in line takes a slot, so a full
# run that waits for the slots to drain keeps every later run from starting. A full ./test.sh (the Integrator's run)
# and targeted runs from the Integrator's integrate* worktrees get priority and go ahead of workers' runs. Tickets of
# dead processes are dropped; a ticket leaves the queue once its run holds its slots.
# Slot 1 is the lock file of the one-lock queue before slots: copies of that version still waiting take only it, and
# their targeted tickets (no SLOT_TICKET_MARK) don't hold up the runs behind them. Install a new version of this file
# by rename (git does, mv does), never by writing into it: bash reads a running script from the file as it goes.
# Targeted runs are niced so the user's apps stay responsive and use at most TARGETED_TEST_THREADS threads
# (RUST_TEST_THREADS); the full suite keeps normal priority, because its task-overlap tests measure wall clock.
LOCK="${WARP_TEST_LOCK:-$HOME/.cargo/warp-tests.lock}"
QUEUE="$LOCK.queue"
TARGETED_SLOTS=3 # one slot left the 10-core Mac at load 7 with 37 runs queued (card test-queue-slots)
SLOT_TICKET_MARK=slots # second line of a ticket written by this version
POLL_SECONDS=2
TARGETED_NICENESS=10
TARGETED_TEST_THREADS=4
HOMEBREW_BIN=/opt/homebrew/bin
# A run with a process using more memory than this is killed: on 2026-10-09 one test grew to 154 GB, swap ran out and
# macOS took down the Claude app with every agent session. Per process, because parallel rustc jobs add up legitimately;
# top's MEM counts compressed memory, ps's RSS doesn't. Compilers and linkers are not capped: rustc building warp-lang
# at opt-level 2 (test.sh) took 4.2–6.2 GB and was killed twice (notes/test_speed.md).
MEMORY_CAP_GB="${WARP_TEST_MEMORY_CAP_GB:-4}"
UNCAPPED_PROGRAMS='rustc|sccache|ld|ld64|ld-prime|clang|cc'
MEMORY_POLL_SECONDS=1
# A test binary running longer than this is killed, naming it and the program it waits for: batch 137 hung 294 s on
# finger_paint.warp (card tests-queue). The tests binary takes ~150 s of a full run, 266 s at most so far.
TEST_TIME_LIMIT_SECONDS="${WARP_TEST_TIME_LIMIT:-600}"
TEST_BINARY_PATTERN='^[^ ]*/deps/[^/ ]+-[0-9a-f]{16}( |$)' # the executable of cargo's test runs: …/deps/<crate>-<hash>
# panics provoked by tests file no to-do board cards (src/diagnostics/crash_card.rs); test.sh runs through here too
export WARP_CRASH_CARDS=0
# test temp files churn on the Junk volume, which fseventsd keeps no log of (notes/churn_volume.md)
JUNK_TMPDIR=/Volumes/Junk/tmp
[ -d "${JUNK_TMPDIR%/*}" ] && mkdir -p "$JUNK_TMPDIR" && export TMPDIR="$JUNK_TMPDIR/"
# rustup's toolchain (nightly, with the wasm32 targets) goes ahead of Homebrew's rust formula, whose cargo and rustc
# win in a PATH that lists /opt/homebrew/bin first (cron, Codex) and lack the wasm32 std (card homebrew-rustc)
RUSTUP_BIN="$HOME/.cargo/bin"
[ -d "$HOMEBREW_BIN" ] && export PATH="$HOMEBREW_BIN:$PATH"
[ -x "$RUSTUP_BIN/rustup" ] && export PATH="$RUSTUP_BIN:$PATH"

# `tests/queue.sh -- --lib x`: cargo's target selection written among the test binary's filters goes before the `--`,
# where cargo reads it; the binary would refuse it as an unknown option (card test-queue)
CARGO_TARGET_FLAGS=" --lib --bins --tests --benches --examples --all-targets --doc "
CARGO_TARGET_FLAGS_WITH_VALUE=" --test --bin --bench --example -p --package "
with_target_flags_first() {
	local before=() after=() past_separator=""
	while [ $# -gt 0 ]; do
		if [ -z "$past_separator" ]; then
			[ "$1" = "--" ] && past_separator=1 || before+=("$1")
		elif [[ "$CARGO_TARGET_FLAGS" == *" $1 "* ]]; then
			before+=("$1")
		elif [[ "$CARGO_TARGET_FLAGS_WITH_VALUE" == *" $1 "* ]] && [ $# -gt 1 ]; then
			before+=("$1" "$2"); shift
		else
			after+=("$1")
		fi
		shift
	done
	run=(cargo --offline test "${before[@]}")
	[ -n "$past_separator" ] && run+=(-- "${after[@]}")
}
if [ "$1" = "cargo" ] || [ -x "$1" ]; then run=("$@"); else with_target_flags_first "$@"; fi
[ -n "$WARP_TEST_LOCKED" ] && exec "${run[@]}"

# lockf is the Mac's (BSD) lock tool, Linux (cloud VMs) has flock; with neither there is no lock to take
if command -v lockf >/dev/null; then
	lock_is_free() { lockf -s -t 0 "$1" true; }; hold_lock=(lockf -k)
elif command -v flock >/dev/null; then
	lock_is_free() { flock -n "$1" true; }; hold_lock=(flock)
else
	echo "test queue: neither lockf nor flock found, running WITHOUT the warp test lock" >&2
	WARP_TEST_LOCKED=1 exec "${run[@]}"
fi
mkdir -p "$(dirname "$LOCK")"

case "${run[0]}" in
	*test.sh) priority=0; niceness=0; exclusive=1 ;;
	*) priority=1; niceness=$TARGETED_NICENESS; exclusive=""; export RUST_TEST_THREADS="${RUST_TEST_THREADS:-$TARGETED_TEST_THREADS}" ;;
esac
# The Integrator's targeted re-runs of a batch's failures hold up main: they go ahead of the workers' runs too
[[ "$PWD" == */warp.worktrees.noindex/integrate* ]] && priority=0

# A `cargo test` builds before it queues: the build waits only for cargo's own lock on the shared target dir, which
# serializes builds anyway, and the test lock is held for running tests, not compiling (a third of its time was builds,
# notes/test_speed.md). The locked run then finds everything fresh.
prebuild_command() {
	[ "${run[0]}" = cargo ] || return 1
	local argument before=()
	for argument in "${run[@]}"; do
		[ "$argument" = "--" ] && break
		[ "$argument" = "--no-run" ] && return 1
		before+=("$argument")
	done
	[[ " ${before[*]} " == *" test "* ]] || return 1
	prebuild=("${before[@]}" --no-run)
}
if prebuild_command; then
	nice -n "$niceness" "${prebuild[@]}" || exit $?
fi

mkdir -p "$QUEUE"
ticket="$QUEUE/$priority-$(date +%s)-$$"
write_ticket() { printf '%s\n%s\n' "$PWD ${run[*]}" "$SLOT_TICKET_MARK" > "$ticket"; }
write_ticket

slot_lock() { [ "$1" -eq 1 ] && echo "$LOCK" || echo "$LOCK.$1"; }
drop_stale_tickets() {
	for t in "$QUEUE"/*; do
		[ -e "$t" ] && ! kill -0 "${t##*-}" 2>/dev/null && rm -f "$t"
	done
}
# Our ticket is found by our PID, whatever its name: a ticket renamed by hand (1- → 0- for priority) once kept its
# waiting run from seeing itself first while it blocked all others with the lock free (2026-10-16, 25 min); a deleted
# one is written again
own_ticket() {
	local name
	name=$(ls "$QUEUE" | grep -E -- "-$$\$" | head -1)
	[ -z "$name" ] && write_ticket && name=${ticket##*/}
	echo "$name"
}
# the tickets before ours that hold us up; a targeted run of an old copy (ticket without SLOT_TICKET_MARK) does not:
# it only ever takes slot 1, the lock file, and keeps its ticket while it runs
tickets_ahead() {
	local own path name mark
	own=$(own_ticket)
	for path in "$QUEUE"/*; do
		name=${path##*/} mark=""
		[ "$name" = "$own" ] && return
		[[ "$name" == 1-* ]] && { read -r _ && read -r mark; } 2>/dev/null < "$path"
		[[ "$name" == 1-* ]] && [ "$mark" != "$SLOT_TICKET_MARK" ] && continue
		echo "$name"
	done
}
# the slots for our run when we are first in line: all of them for a full run, else a free one (slot 1 last, which old
# copies' runs compete for); none free → fails
pick_slots() {
	local slot
	slots=()
	[ -z "$(tickets_ahead)" ] || return 1
	for slot in $(seq 2 "$TARGETED_SLOTS") 1; do
		if lock_is_free "$(slot_lock "$slot")"; then
			slots+=("$(slot_lock "$slot")")
			[ -z "$exclusive" ] && return 0
		elif [ -n "$exclusive" ]; then
			return 1
		fi
	done
	[ ${#slots[@]} -gt 0 ]
}
slot_holders() {
	local slot
	for slot in $(seq 1 "$TARGETED_SLOTS"); do
		lock_is_free "$(slot_lock "$slot")" || cat "$(slot_lock "$slot").owner" 2>/dev/null || echo "unknown holder"
	done | paste -sd ';' - | sed 's/;/; /g'
}

drop_stale_tickets
if ! pick_slots; then
	echo "test queue: waiting for a warp test slot (busy: $(slot_holders); $(tickets_ahead | wc -l | tr -d ' ') ahead" \
		"of us)" >&2
	while sleep $POLL_SECONDS; drop_stale_tickets; ! pick_slots; do :; done
fi

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
# the given PIDs but compilers and linkers (UNCAPPED_PROGRAMS)
capped() {
	ps -o pid=,command= -p "$(IFS=,; echo "$*")" 2>/dev/null | awk '{ print $1, $2 }' |
		grep -vE " ([^ ]*/)?($UNCAPPED_PROGRAMS)$" | cut -d' ' -f1
}
watch_run() {
	local run_pid=$1 cap_mb=$((MEMORY_CAP_GB * 1024)) pids used_mb
	while kill -0 "$run_pid" 2>/dev/null; do
		sleep $MEMORY_POLL_SECONDS
		pids=($(descendants "$run_pid"))
		[ ${#pids[@]} -eq 0 ] && continue
		kill_overdue_test_binaries "${pids[@]}"
		[ "$(uname)" = Darwin ] || continue
		used_mb=$(largest_memory_mb $(capped "${pids[@]}"))
		if [ "$used_mb" -gt "$cap_mb" ]; then
			echo "test queue: KILLED the run, a process used ${used_mb} MB, over the memory cap of ${MEMORY_CAP_GB} GB" \
				"(WARP_TEST_MEMORY_CAP_GB): ${run[*]}" >&2
			kill -9 "${pids[@]}" 2>/dev/null
			return
		fi
	done
}
watch_run $$ &

# exec keeps our PID, so the ticket stays valid until the lockf of every slot holds; then the ticket goes, found by PID
# like own_ticket finds it, so the runs behind us move up, and each slot's owner file names us
export WARP_TEST_LOCKED=1
export WARP_NO_WINDOW=1 # a painting program writes PNGs, never a window during tests (src/graphics/paint.rs)
hold_slots=()
for slot in "${slots[@]}"; do hold_slots+=("${hold_lock[@]}" "$slot"); done
exec "${hold_slots[@]}" bash -c 'pid=$1 queue=$2 slots=$3; shift 3; rm -f "$queue"/*-"$pid"
	for slot in $slots; do echo "$pid $PWD $*" > "$slot.owner"; done; exec "$@"' \
	queue-run "$$" "$QUEUE" "${slots[*]}" nice -n "$niceness" "${run[@]}"
