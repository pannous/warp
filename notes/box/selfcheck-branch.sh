#!/bin/bash
# usage: /workspace/box-slot.sh /workspace/selfcheck-branch.sh <branch>
# Fetches origin/<branch> into /workspace/cow/trees/sc-<branch> and runs the pure+self self-check.
# Always run it through box-slot.sh (3 heavy slots); it takes no lock of its own, so
# different branches can check in parallel. Uses /workspace/cow/warp-rust-latest (refresh: /workspace/box-slot.sh /workspace/build-fresh-warp.sh).
br=$1; [ -z "$br" ] && { echo "usage: $0 <branch>"; exit 2; }
repo=/workspace/wt-main
wt=/workspace/cow/trees/sc-$br
exec 8>/workspace/selfcheck-lock/sc-$br.lock; flock 8   # one self-check per branch
cd $repo && git fetch -q origin "+refs/heads/$br:refs/remotes/origin/$br" || exit 1
tip=$(git -C $repo rev-parse "refs/remotes/origin/$br") || exit 1   # never FETCH_HEAD: linked worktrees keep a stale one
if [ -d $wt ]; then cd $wt && git checkout -q --detach "$tip" || exit 1
else mkdir -p /workspace/cow/trees && git -C $repo worktree add -q --detach $wt "$tip" || exit 1; fi
cd $wt && echo "== tip $(git rev-parse --short HEAD)"
sed -e "s#^wt=.*#wt=$wt#" -e "s#^warp=.*#warp=/workspace/selfcheck-lock/warp-$br#" /workspace/selfcheck-box.sh > /tmp/sc-$br.sh && bash /tmp/sc-$br.sh
