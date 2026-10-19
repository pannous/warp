#!/bin/sh
# A crash of a build older than its checkout's src/ or lib/ files no card (card crash-4: a binary built before the
# wasm-body parser lowered the pulled runtime.warp, read `wasm f64.nearest` as nearest(f64) and filed a crash card).
# Run in a clean worktree after scripts/own-warp.sh: a temporary branch commits a runtime.warp that does not lower,
# so scratch/warp is older than its checkout and crashes under WARP_SELF_HOSTED; expected on stderr: "no crash card".
set -e
cd "$(git rev-parse --show-toplevel)"
test -z "$(git status --porcelain --untracked-files=no)" || { echo "needs a clean worktree"; exit 2; }
branch=$(git branch --show-current)
probe_branch="$branch-stale-probe"
git checkout -q -b "$probe_branch"
sed -i '' 's/:= wasm f64.nearest/:= nearest(x)/' lib/compiler/runtime.warp
git commit -q -am "probe: runtime.warp that does not lower"
output=$(WARP_NO_WINDOW=1 WARP_SELF_HOSTED=1 scratch/warp '1 + 2' 2>&1 || true)
git checkout -q "$branch"
git branch -q -D "$probe_branch"
echo "$output" | grep -E "does not lower|crash card|filing a card"
echo "$output" | grep -q "no crash card: this build" && echo "ok: stale build files no card"
