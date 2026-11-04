#!/usr/bin/env bash
# scripts/integrate/on-push.sh: the pre-push hook of every warp checkout and worktree (.git/hooks/pre-push execs it).
# A push to main starts after-push.sh once the pushed commit has arrived on origin's main, detached, so the push
# itself never waits. This replaced the cron that polled main every 2 minutes and kept rebuilding (user 2026-11-04:
# "make that part of the merge script"). after-push.sh holds a lock and installs main's newest tip, so pushes in quick
# succession share one run. A refused push never reaches main and starts nothing.
ARRIVAL_POLLS=36   # every 5 s: three minutes for the push to finish
POLL_SECONDS=5
LOG=$HOME/Library/Logs/warp/after-push.log
# the main checkout: a worker's worktree may be gone a moment after its push
MAIN_CHECKOUT=$(cd "$(dirname "$0")/../.." && pwd)
AFTER_PUSH=$MAIN_CHECKOUT/scripts/integrate/after-push.sh
NO_COMMIT=0000000000000000000000000000000000000000
# the hook runs inside the pushing worktree's git environment (GIT_DIR, GIT_INDEX_FILE …): left set, every git call
# below and in after-push.sh acts on that worktree, gone a moment after its push (2026-11-04 20:21: the tree stayed on
# the old tip and native-2ad9eb2dc got the old compilers, "unknown name protocol")
unset $(git rev-parse --local-env-vars)

on_main() {
	git -C "$MAIN_CHECKOUT" fetch -q origin main 2>/dev/null &&
		git -C "$MAIN_CHECKOUT" merge-base --is-ancestor "$1" origin/main
}

install_when_arrived() {
	local pushed=$1
	for ((poll = 0; poll < ARRIVAL_POLLS; poll++)); do
		sleep "$POLL_SECONDS"
		on_main "$pushed" && exec "$AFTER_PUSH"
	done
	echo "on-push: ${pushed:0:10} never reached main, after-push not started"
}

while read -r _ local_sha remote_ref _; do
	[ "$remote_ref" = refs/heads/main ] && [ "$local_sha" != "$NO_COMMIT" ] || continue
	mkdir -p "$(dirname "$LOG")"
	nohup bash -c "$(declare -f on_main install_when_arrived); $(declare -p MAIN_CHECKOUT ARRIVAL_POLLS POLL_SECONDS AFTER_PUSH);
		install_when_arrived $local_sha" >>"$LOG" 2>&1 </dev/null &
	echo "on-push: after-push.sh starts once ${local_sha:0:10} is on main (log $LOG)"
done
exit 0
