#!/usr/bin/env bash
# scripts/promote-self.sh [--background] [<worktree>, default this checkout]: the fixpoint test of the tree's warp.wasm,
# promoting it to warp-self.wasm (`warp self`'s compiler) only on a pass. warp.wasm (stage N) compiles
# core/compiler.warp again (stage N+1); only output byte-identical to warp.wasm's own module copies warp.wasm to
# data/warp-self.wasm (`warp rebuild self`, src/compiler/standalone.rs promote; through scripts/rebuild.sh self, so a
# promoted warp-self.wasm of these compiler inputs in data/ or the store is taken without running the test again, and a
# new one is published to the store). A missing warp.wasm of these inputs is built first.
# Never fails its caller: it prints PROMOTED, NO FIXPOINT or FAILED and exits 0. Meant to run every once in a while,
# in parallel, without blocking anything: --background detaches it (log data/promote-self.log); a run already going for
# the tree makes another one a no-op (lock folder data/promote-self.lock). The normal rebuild and the gate need only warp.wasm.
# WARP_BINARY: the warp binary (default as scripts/rebuild.sh: the shared one).
set -uo pipefail
background=
[ "${1-}" = --background ] && { background=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 0
scripts=$(cd "$(dirname "$0")" && pwd)
data=$tree/data
mkdir -p "$data" || exit 0
log=$data/promote-self.log
if [ -n "$background" ]; then
	# a session of its own (setsid where there is one, the box): the caller's end never takes it along
	detach=(nohup)
	command -v setsid >/dev/null && detach=(setsid nohup)
	"${detach[@]}" "$0" "$tree" >>"$log" 2>&1 </dev/null &
	echo "promote-self: running in the background (pid $!), log $log"
	exit 0
fi
# one run per tree: a lock folder holding the runner's pid (mkdir is atomic; no flock on the Mac); a dead runner's is taken over
lock=$data/promote-self.lock
if ! mkdir "$lock" 2>/dev/null; then
	holder=$(cat "$lock/pid" 2>/dev/null)
	if [ -n "$holder" ] && kill -0 "$holder" 2>/dev/null; then echo "promote-self: run $holder for $tree is going; skipped"; exit 0; fi
	rm -rf "$lock" && mkdir "$lock" 2>/dev/null || { echo "promote-self: lock $lock taken; skipped"; exit 0; }
fi
echo $$ >"$lock/pid"
trap 'rm -rf "$lock"' EXIT
started=$(date +%s)
echo "promote-self: $(date '+%Y-%m-%d %H:%M:%S %Z') fixpoint test of $tree/data/warp.wasm"
# a compile of compiler.warp runs for minutes: an exported short limit must not cut it
said=$(BOX_SLOT_VERBOSE=1 WARP_TIME_LIMIT=${WARP_PROMOTE_TIME_LIMIT:-7200} "$scripts/rebuild.sh" self "$tree" 2>&1)
status=$?
took=$(($(date +%s) - started))
sha() { { sha256sum "$1" 2>/dev/null || shasum -a 256 "$1"; } | cut -c1-16; }
if [ $status = 0 ]; then
	echo "promote-self: PROMOTED in ${took}s: warp-self.wasm = warp.wasm (sha256 $(sha "$data/warp-self.wasm")…)"
elif echo "$said" | grep -q 'no fixpoint'; then
	echo "promote-self: NO FIXPOINT in ${took}s: $(echo "$said" | grep -o 'no fixpoint[^"]*' | head -1 | cut -c1-240)"
else
	echo "promote-self: FAILED in ${took}s (warp-self.wasm left as it was): $(echo "$said" | tail -3 | tr '\n' ' ' | cut -c1-400)"
fi
exit 0
