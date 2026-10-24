#!/bin/bash
# every 10 s: what the warp test-lock holder's process tree is doing (building: rustc/ld/cargo-only, running: a test binary)
LOCK=$HOME/.cargo/warp-tests.lock
OUT="$(dirname "$0")/../../data/perf/queue_samples.log"
descendants() { local c; for c in $(pgrep -P "$1"); do echo "$c"; descendants "$c"; done; }
while true; do
	owner=$(cut -d' ' -f1 "$LOCK.owner" 2>/dev/null)
	state=idle
	if [ -n "$owner" ] && kill -0 "$owner" 2>/dev/null; then
		cmds=$(ps -o command= -p "$(descendants "$owner" | paste -sd, -)" 2>/dev/null)
		if echo "$cmds" | grep -qE '/deps/[^/ ]+-[0-9a-f]{16}( |$)'; then state=running
		elif echo "$cmds" | grep -qE 'rustc|/ld |ld64|sccache|cargo'; then state=building
		else state=other; fi
	fi
	echo "$(date +%s) $state $(ls $LOCK.queue | wc -l | tr -d ' ') $(cut -d' ' -f2 "$LOCK.owner" 2>/dev/null)" >> "$OUT"
	sleep 10
done
