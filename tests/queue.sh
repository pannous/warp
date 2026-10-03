#!/bin/bash
# The one gate for running warp tests on this machine (notes/roles.md, "Tester"): every test run waits for one
# machine-wide lock, so at most one test binary runs at a time; the others queue instead of overloading the Mac.
#   tests/queue.sh [cargo test args]      e.g. tests/queue.sh -- test_traits      (cargo --offline test …)
#   tests/queue.sh cargo browser-test x   any cargo command
#   tests/queue.sh ./test.sh              a script (test.sh re-enters through here by itself)
# A run that already holds the lock (WARP_TEST_LOCKED) runs directly, so nesting never deadlocks.
LOCK="$HOME/.cargo/warp-tests.lock"

if [ "$1" = "cargo" ] || [ -x "$1" ]; then run=("$@"); else run=(cargo --offline test "$@"); fi
[ -n "$WARP_TEST_LOCKED" ] && exec "${run[@]}"

if ! lockf -s -t 0 "$LOCK" true; then
	echo "test queue: waiting for the warp test lock ($(cat "$LOCK.owner" 2>/dev/null || echo unknown holder))" >&2
fi
export WARP_TEST_LOCKED=1
exec lockf -k "$LOCK" bash -c 'echo "$PPID $PWD $*" > "$0.owner"; "$@"' "$LOCK" "${run[@]}"
