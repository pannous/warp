#!/bin/zsh
# The nightly tier (user 2026-10-16, notes/test_speed.md): tests the gate only samples run in full on the latest
# origin/main, in a detached worktree of their own, through the test queue. A failure becomes a to-do card with the
# log's path. Run by cron at night:
#   10 3 * * * /Users/me/dev/angles/warp/tests/nightly.sh >> /Users/me/.companion/logs/warp-nightly.log 2>&1
# the main checkout, also when this script runs from a worktree
WARP=$(git -C ${0:A:h} worktree list --porcelain | head -1 | cut -d' ' -f2-)
WORKTREE=$WARP:h/warp.worktrees.noindex/nightly
LOGS=$WARP/data/nightly
# the tests that check more with WARP_NIGHTLY set: the self-hosting twins (tests/common/twins.rs, every 4th source in
# the gate)
NIGHTLY_TESTS=(parser::test_self_)
VERSION_SUFFIX=nightly
export PATH=$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:$HOME/dev/bin:$PATH
# todo (the board) under cron, as the other warp cron jobs do
export GH_TOKEN=${GH_TOKEN:-$(cat $HOME/.config/gh/cron-token 2>/dev/null)}

git -C $WARP fetch -q origin || { echo "nightly: fetch failed"; exit 1; }
if [[ ! -d $WORKTREE ]]; then
	cowtree add $WORKTREE --detach origin/main || exit 1
fi
cd $WORKTREE || exit 1
# the version tweak keeps this worktree's artifacts apart from other checkouts' in the shared target dir; restored
# before moving to the new main
$WARP/scripts/version-tweak.sh restore
git checkout -q --detach origin/main || exit 1
$WARP/scripts/version-tweak.sh $VERSION_SUFFIX > /dev/null

mkdir -p $LOGS
log=$LOGS/$(date +%F).log
commit=$(git rev-parse --short HEAD)
echo "$(date '+%F %T') nightly on $commit: ${NIGHTLY_TESTS[*]}"
WARP_NIGHTLY=1 tests/queue.sh --all-features --test tests -- $NIGHTLY_TESTS > $log 2>&1
result=$?
failed=(${(f)"$(grep -E '^test .* FAILED' $log | awk '{print $2}')"})
summary=$(grep '^test result' $log | tail -1)
echo "$(date '+%F %T') exit $result: $summary"
if (( result != 0 )); then
	todo add "nightly tests failed on $commit: ${failed[*]:-the run (exit $result)}; log $log" ||
		echo "nightly: todo add failed"
fi
