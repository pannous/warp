#!/usr/bin/env bash
# scripts/integrate/land.sh [--tree <worktree>]: step 2 of landing the branches of retired sessions, after merge.sh's
# tier 0 went green. Pushes exactly the tested hash to main, closes each merged card with its merge commit, deletes its
# remote branch, then watches the push like any other (after-push.sh, in the background).
set -uo pipefail
DEFAULT_TREE=/Volumes/Worktrees/integrate

tree=$DEFAULT_TREE
[ "${1-}" = --tree ] && { tree=$2; shift 2; }
tree=$(cd "$tree" && pwd) || exit 1
batch=$tree/data/integrate/batch
say() { echo "land: $*"; }
field() { sed -n "s/^$1 //p" "$batch"; }

tested=$(field tested) base=$(field base)
[ -n "$tested" ] || { say "no batch in $batch: run merge.sh first"; exit 1; }
[ "$(git -C "$tree" rev-parse HEAD)" = "$tested" ] || { say "the tree moved off the tested ${tested:0:10}"; exit 1; }
git -C "$tree" push -q origin "$tested:refs/heads/main" || { say "push refused (main moved?): run merge.sh again"; exit 1; }
say "pushed ${tested:0:10} to main"

while read -r key branch ref; do
	git -C "$tree" merge-base --is-ancestor "$ref" "$tested" || { say "NOT MERGED $key ($ref)"; continue; }
	commit=$(git -C "$tree" log --merges --format=%H --grep="(card $key)" -1 "$base..$tested")
	todo done "$key" "$commit" >/dev/null && say "done $key"
	git -C "$tree" push -q origin --delete "$branch" 2>/dev/null
done < <(field card)

log=$tree/data/integrate/after-push-${tested:0:10}.log
nohup "$(dirname "$0")/after-push.sh" "$tested" >"$log" 2>&1 </dev/null &
say "after-push.sh running in the background: $log"
