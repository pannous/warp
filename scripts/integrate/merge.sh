#!/usr/bin/env bash
# scripts/integrate/merge.sh [--tree <worktree>] [--tip <commit>] [<card>…]: landing the branches of retired sessions,
# step 1 of 2 (step 2: land.sh). --tip: a merged tip handed over (a carrier branch), checked out in the tree for tier 0 alone.
# Else it merges the cards noted "land: <branch> <tip>" (default: all of `todo landing`) onto origin/main in the tree,
# one merge commit each,
# then runs tier 0 on the result: the compiler of the merged lib (scripts/rebuild.sh pure: taken from data/ or the
# store when its inputs were built before, so a tip already built is never built twice), hello through it, and
# scripts/self-check.sh when the batch touches core/. A card whose merge conflicts is left out and named.
# The batch (tested hash, base, cards) goes to <tree>/data/integrate/batch for land.sh.
# Exit: 0 tier 0 green, 1 red or nothing merged.
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp
DEFAULT_TREE=/Volumes/Worktrees/integrate
EXPECTED=Hello

tree=$DEFAULT_TREE tip=
[ "${1-}" = --tree ] && { tree=$2; shift 2; }
[ "${1-}" = --tip ] && { tip=$2; shift 2; }
tree=$(cd "$tree" && pwd) || exit 1
scripts=$(cd "$(dirname "$0")/.." && pwd)
batch=$tree/data/integrate/batch
mkdir -p "$(dirname "$batch")"
say() { echo "merge: $*"; }

git -C "$tree" fetch -q origin || exit 1
git -C "$tree" checkout -q --detach origin/main || exit 1
base=$(git -C "$tree" rev-parse HEAD)
[ -n "$tip" ] && { git -C "$tree" checkout -q --detach "$tip" || exit 1; }

landing=$([ -n "$tip" ] && echo '[]' || todo landing --json) || exit 1
[ $# -gt 0 ] && landing=$(jq --args '[.[] | select(.key as $key | $ARGS.positional | index($key))]' "$@" <<<"$landing")

# the ref a card lands at: origin/<branch> when it descends from the tip the card names, else that tip
landing_ref() {
	local branch=$1 tip=$2
	git -C "$tree" merge-base --is-ancestor "$tip" "origin/$branch" 2>/dev/null && echo "origin/$branch" || echo "$tip"
}

merged=()
while IFS=$'\t' read -r key branch card_tip title; do
	ref=$(landing_ref "$branch" "$card_tip")
	summary=$(git -C "$tree" log --no-merges --format='- %s' "HEAD..$ref")
	if git -C "$tree" merge -q --no-ff -m "Merge branch '$branch' (card $key): $title" -m "$summary" "$ref"; then
		merged+=("$key $branch $(git -C "$tree" rev-parse "$ref")")
		say "merged $key ($branch at ${ref})"
	else
		say "CONFLICT $key ($branch): $(git -C "$tree" diff --name-only --diff-filter=U | tr '\n' ' ')"
		git -C "$tree" merge --abort
	fi
done < <(jq -r '.[] | [.key, .branch, .tip, .title] | @tsv' <<<"$landing")
((${#merged[@]})) || [ -n "$tip" ] || { say "nothing merged"; exit 1; }

tested=$(git -C "$tree" rev-parse HEAD)
{ echo "tested $tested"; echo "base $base"; for card in "${merged[@]+"${merged[@]}"}"; do echo "card $card"; done; } >"$batch"

"$scripts/rebuild.sh" pure "$tree" || { say "RED: rebuild pure failed"; exit 1; }
got=$(cd "$tree" && WARP_NO_WINDOW=1 WARP_LIB=$tree/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
[[ $got == *$EXPECTED* ]] || { say "RED: pure hello got ${got:0:160}"; exit 1; }
if ! git -C "$tree" diff --quiet "$base" HEAD -- core; then
	"$scripts/self-check.sh" "$tree" || { say "RED: self-check"; exit 1; }
fi
say "tier 0 green: ${#merged[@]} cards, tested ${tested:0:10}; land with scripts/integrate/land.sh --tree $tree"
