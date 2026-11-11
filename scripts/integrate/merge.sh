#!/usr/bin/env bash
# scripts/integrate/merge.sh [--tree <worktree>] [--tip <commit>] [--smoke-only] [<card>…]: landing the branches of retired sessions,
# step 1 of 2 (step 2: land.sh). --tip: a merged tip handed over (a carrier branch), checked out in the tree for tier 0 alone.
# Else it merges the cards noted "land: <branch> <tip>" (default: all of `todo landing`) onto origin/main in the tree,
# one merge commit each,
# then runs tier 0 on the result: the compiler of the merged lib (scripts/rebuild.sh pure: taken from data/ or the
# store when its inputs were built before, so a tip already built is never built twice), hello through it,
# scripts/self-check.sh when the batch touches core/ and the page smoke (--smoke-only: neither; land.sh's retry of a
# batch already checked, the new main commits being tested by their pushers, night decision 00:50 2026-11-03).
# A card whose merge conflicts is left out and named.
# A red batch is split in halves (card red-batch-split): each half is tried beside the cards already green, a red half
# is halved again, so a red card alone is found in about log2 N extra tier 0 runs; the green cards stay in the batch,
# each red one goes back to Now with its error as a card note.
# The batch (tested hash, base, cards) goes to <tree>/data/integrate/batch for land.sh.
# Exit: 0 tier 0 green on at least one card, 1 red or nothing merged. Nothing green puts the tree back on origin/main and
# drops the batch (the branches keep their merges), so no failed merge is left looking like a batch waiting to land
# (integrator-watch).
set -uo pipefail
MAIN_WARP=$HOME/dev/bin/warp
DEFAULT_TREE=/Volumes/Worktrees/integrate
EXPECTED=Hello

tree=$DEFAULT_TREE tip=
[ "${1-}" = --tree ] && { tree=$2; shift 2; }
[ "${1-}" = --tip ] && { tip=$2; shift 2; }
smoke_only=
[ "${1-}" = --smoke-only ] && { smoke_only=1; shift; }
tree=$(cd "$tree" && pwd) || exit 1
scripts=$(cd "$(dirname "$0")/.." && pwd)
batch=$tree/data/integrate/batch
mkdir -p "$(dirname "$batch")"
say() { echo "merge: $*"; }
red() { say "RED: $*"; git -C "$tree" checkout -q --detach "$base"; rm -f "$batch"; exit 1; }

git -C "$tree" fetch -q origin || exit 1
git -C "$tree" checkout -q --detach origin/main || exit 1
base=$(git -C "$tree" rev-parse HEAD)

# tier 0 on the tree's HEAD; on red it names the failure in $failure and returns 1
tier0() {
	"$scripts/rebuild.sh" pure "$tree" || { failure="rebuild pure failed"; return 1; }
	local got
	got=$(cd "$tree" && WARP_WINDOW=0 timeout 300 "$MAIN_WARP" --lib "$tree/lib" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { failure="pure hello got ${got:0:160}"; return 1; }
	[ -n "$smoke_only" ] && return 0
	if ! git -C "$tree" diff --quiet "$base" HEAD -- core extensions; then
		"$scripts/self-check.sh" "$tree" || { failure=self-check; return 1; }
	fi
	got=$("$scripts/page-smoke.sh" "$tree") || { failure=${got:0:180}; return 1; }
}

if [ -n "$tip" ]; then
	git -C "$tree" checkout -q --detach "$tip" || exit 1
	tier0 || red "$failure"
	{ echo "tested $(git -C "$tree" rev-parse HEAD)"; echo "base $base"; } >"$batch"
	say "tier 0 green: tested ${tip:0:10}; land with scripts/integrate/land.sh --tree $tree"
	exit 0
fi

landing=$(todo landing --json) || exit 1
[ $# -gt 0 ] && landing=$(jq --args '[.[] | select(.key as $key | $ARGS.positional | index($key))]' "$@" <<<"$landing")
rows=()
while IFS= read -r row; do rows+=("$row"); done < <(jq -r '.[] | [.key, .branch, .tip, .title] | @tsv' <<<"$landing")

# the ref a card lands at: origin/<branch> when it descends from the tip the card names, else that tip
landing_ref() {
	local branch=$1 tip=$2
	git -C "$tree" merge-base --is-ancestor "$tip" "origin/$branch" 2>/dev/null && echo "origin/$branch" || echo "$tip"
}

# merges the given rows onto base, one merge commit each; a card that conflicts is named and left out for good
conflicted=()
merge_rows() {
	git -C "$tree" checkout -q --detach "$base"
	merged_rows=()
	local i key branch card_tip title ref summary
	for i in "$@"; do
		[[ " ${conflicted[*]-} " == *" $i "* ]] && continue
		IFS=$'\t' read -r key branch card_tip title <<<"${rows[i]}"
		ref=$(landing_ref "$branch" "$card_tip")
		summary=$(git -C "$tree" log --no-merges --format='- %s' "HEAD..$ref")
		if git -C "$tree" merge -q --no-ff -m "Merge branch '$branch' (card $key): $title" -m "$summary" "$ref"; then
			merged_rows+=("$i")
		else
			say "CONFLICT $key ($branch): $(git -C "$tree" diff --name-only --diff-filter=U | tr '\n' ' ')"
			git -C "$tree" merge --abort
			conflicted+=("$i")
		fi
	done
}

# tries the rows beside the green ones: green joins them, a red single row is a culprit, a red group is halved
green=() culprits=() green_head=
try_rows() {
	merge_rows "${green[@]+"${green[@]}"}" "$@"
	((${#merged_rows[@]} > ${#green[@]})) || return 0
	if tier0; then
		green=("${merged_rows[@]}") green_head=$(git -C "$tree" rev-parse HEAD)
		say "green: $(row_keys "${green[@]}")"
	elif (($# == 1)); then
		culprits+=("$1")
		local key=${rows[$1]%%$'\t'*}
		say "RED $key: $failure"
		todo move "$key" Now >/dev/null && todo note "$key" "landing: red in merge.sh tier 0 beside the green cards ($failure); fix and hand over again" >/dev/null
	else
		say "red with $(row_keys "$@"): $failure; halving"
		local half=$(($# / 2))
		try_rows "${@:1:half}"
		try_rows "${@:half+1}"
	fi
}
row_keys() { local i; for i in "$@"; do printf '%s ' "${rows[i]%%$'\t'*}"; done; }

((${#rows[@]})) || { say "nothing to merge"; exit 1; }
try_rows $(seq 0 $((${#rows[@]} - 1)))
((${#green[@]})) || red "no card green${culprits[*]+; red: $(row_keys "${culprits[@]}")}"
if [ "$(git -C "$tree" rev-parse HEAD)" != "$green_head" ]; then
	git -C "$tree" checkout -q --detach "$green_head"
	"$scripts/rebuild.sh" pure "$tree" || red "rebuild pure of the green cards failed"
fi
{
	echo "tested $green_head"
	echo "base $base"
	for i in "${green[@]}"; do
		IFS=$'\t' read -r key branch card_tip title <<<"${rows[i]}"
		echo "card $key $branch $(git -C "$tree" rev-parse "$(landing_ref "$branch" "$card_tip")")"
	done
} >"$batch"
((${#culprits[@]})) && say "sent back: $(row_keys "${culprits[@]}")"
say "tier 0 green: ${#green[@]} cards, tested ${green_head:0:10}; land with scripts/integrate/land.sh --tree $tree"
