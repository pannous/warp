#!/bin/zsh
# Removes finished agent worktrees from the Worktrees volume (WORKTREE_ROOTS) and the branches merged
# into origin/main, which nobody
# else removes (54 worktrees, 56 local and 39 remote branches on 2026-10-14). Nothing unmerged is touched:
#   1. a worktree on a branch whose HEAD origin/main holds (also as equal patches after a rebase or squash), idle for
#      IDLE_HOURS, whose only uncommitted changes are the Cargo version tweak (TWEAK_FILES) and with no untracked
#      files: removed with its branch
#   2. local and remote branches origin/main holds the same way, with no worktree checked out and a tip older than
#      IDLE_HOURS
#   3. loose files in the worktrees folder (logs, patches, dumps) older than LEFTOVER_DAYS: moved to data/
# Detached worktrees (integrate, main-binary …) are never removed; DONOR, which cowtree clones new worktrees from on
# the Worktrees volume (notes/churn_volume.md), is moved to origin/main when clean. Skips for untracked files are listed.
#   prune-worktrees.sh            prune, print what it did
#   prune-worktrees.sh --dry-run  only print what it would do
# Run daily by cron.
WARP=/Users/me/dev/angles/warp
# worktrees live on the Worktrees volume (notes/churn_volume.md); integrator/ there is a plain folder, not a worktree
WORKTREE_ROOTS=(/Volumes/Worktrees)
DONOR=/Volumes/Worktrees/warp  # named after the project: the volume may hold other projects
LEFTOVERS=$WARP/data/worktree-leftovers
TWEAK_FILES=(Cargo.toml Cargo.lock crates/warp-runtime/Cargo.toml)
IDLE_HOURS=${IDLE_HOURS:-6}
LEFTOVER_DAYS=2
KEPT_BRANCHES=(main HEAD)

dry_run=$([[ $1 == --dry-run ]] && echo yes)
git=(git -C $WARP)

act() {  # print the action, run it unless dry run
  echo "$(date '+%F %T') $*"
  [[ -n $dry_run ]] || "$@" >/dev/null 2>&1 || echo "  failed: $*"
}

merged() {  # origin/main holds the commit or, after a rebase or squash merge, an equal patch of each commit
  $git merge-base --is-ancestor $1 origin/main 2>/dev/null || ! $git cherry origin/main $1 | grep -q '^+'
}

recently_written() {  # any file in the worktree written within IDLE_HOURS
  [[ -n $(/usr/bin/find $1 -path $1/.git -prune -o -type f -mmin -$(( IDLE_HOURS * 60 )) -print -quit) ]]
}

in_worktree_roots() { for root in $WORKTREE_ROOTS; do [[ $1 == $root/* ]] && return 0; done; return 1 }

old_tip() { (( $(date +%s) - $($git log -1 --format=%ct $1) > IDLE_HOURS * 3600 )) }

$git fetch --prune --quiet origin

checked_out=()
$git worktree list --porcelain | awk '/^worktree /{path=$2} /^branch /{sub("refs/heads/","",$2); print path, $2}' |
while read -r worktree branch; do
  in_worktree_roots $worktree || { checked_out+=($branch); continue; }
  worktree_status=$(git -C $worktree status --porcelain 2>/dev/null)
  changed=$(print -r -- $worktree_status | grep -v '^??' | cut -c4- | grep -vxF -f <(print -l $TWEAK_FILES))
  if ! merged $(git -C $worktree rev-parse HEAD) || [[ -n $changed ]] || recently_written $worktree; then
    checked_out+=($branch)
  elif print -r -- $worktree_status | grep -q '^??'; then
    echo "$(date '+%F %T') kept for untracked files: $worktree"
    checked_out+=($branch)
  else
    act $git worktree remove --force $worktree
  fi
done

for branch in $($git for-each-ref --format='%(refname:short)' refs/heads); do
  (( ${KEPT_BRANCHES[(Ie)$branch]} || ${checked_out[(Ie)$branch]} )) && continue
  merged $branch && old_tip $branch && act $git branch -D $branch
done
for remote in $($git for-each-ref --format='%(refname:lstrip=3)' refs/remotes/origin); do
  (( ${KEPT_BRANCHES[(Ie)$remote]} || ${checked_out[(Ie)$remote]} )) && continue
  merged origin/$remote && old_tip origin/$remote && act $git push --quiet origin --delete $remote
done

# the donor follows main, so new clones share most files with it
if [[ -d $DONOR && -z $(git -C $DONOR status --porcelain --untracked-files=no 2>/dev/null) ]]; then
  [[ $(git -C $DONOR rev-parse HEAD) == $($git rev-parse origin/main) ]] || act git -C $DONOR checkout --quiet --detach origin/main
fi

mkdir -p $LEFTOVERS
/usr/bin/find ${^WORKTREE_ROOTS}(N/) -maxdepth 1 -type f -mtime +$LEFTOVER_DAYS | while read -r file; do act mv $file $LEFTOVERS/; done
