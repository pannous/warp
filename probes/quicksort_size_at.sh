#!/bin/bash
# probes/quicksort_size_at.sh <commit>: size of samples/quicksort.warp's wasm built at that commit (card samples-quicksort);
# builds in the detached worktree samples-quicksort-bisect with its own version, so no other build is disturbed
set -euo pipefail
tree=/Users/me/dev/angles/warp.worktrees.noindex/samples-quicksort-bisect
git -C $tree checkout -q --detach -f "$1"
perl -0pi -e 's/^version = "[^"]+"/version = "9.9.9-qsbisect"/m' $tree/Cargo.toml $tree/crates/warp-runtime/Cargo.toml
perl -pi -e 's/(warp-runtime = \{ path = "crates\/warp-runtime", version = ")[^"]+/${1}9.9.9-qsbisect/' $tree/Cargo.toml
cd $tree && CARGO_BUILD_JOBS=2 scripts/own-warp.sh >/dev/null 2>&1 || { echo "$1 build failed"; exit 1; }
cp $tree/samples/quicksort.warp $tree/qs.warp
echo "$1 $(git -C $tree log -1 --format=%ad --date=short) $($tree/scratch/warp build --wasm $tree/qs.warp 2>&1 | grep compiled)"
