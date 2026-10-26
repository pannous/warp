#!/usr/bin/env bash
# scripts/integrate/land.sh [--tree <worktree>]: step 2 of the Integrator's batch, after merge.sh's tier 0 went green.
# Pushes exactly the tested hash to main, closes each merged card with its merge commit and deletes its remote branch,
# installs the tree's compilers beside the shared warp binary, brings the user's checkout up to date, and starts the
# slow tiers (gate.sh tiers 1-2, promote-self.sh) in the background: they never hold a merge.
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
DEFAULT_TREE=/Volumes/Worktrees/integrate
SLOW_TREE=/Volumes/Worktrees/integrate2
EXPECTED=Hello

tree=$DEFAULT_TREE
[ "${1-}" = --tree ] && { tree=$2; shift 2; }
tree=$(cd "$tree" && pwd) || exit 1
scripts=$(cd "$(dirname "$0")/.." && pwd)
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

# the tree's compilers beside the shared binary, warp-self.wasm only when promoted from the same inputs
install_compilers() {
	local folder data=$tree/data inputs
	folder=$(dirname "$(readlink -f "$MAIN_WARP")")
	"$scripts/rebuild.sh" both "$tree" >/dev/null || return 1
	inputs=$(cut -d' ' -f1 "$data/warp.wasm.inputs")
	cp "$data/warp.wasm" "$folder/warp.wasm.new" && mv "$folder/warp.wasm.new" "$folder/warp.wasm"
	if [ -f "$data/warp-self.wasm" ] && [ "$(cut -d' ' -f1 "$data/warp-self.wasm.inputs" 2>/dev/null)" = "$inputs" ]; then
		cp "$data/warp-self.wasm" "$folder/warp-self.wasm.new" && mv "$folder/warp-self.wasm.new" "$folder/warp-self.wasm"
	fi
	local lane got
	for lane in pure self; do
		[ $lane = self ] && [ ! -f "$folder/warp-self.wasm" ] && continue
		got=$(cd "$MAIN_CHECKOUT" && WARP_NO_WINDOW=1 timeout 300 "$MAIN_WARP" $lane samples/hello/hello.warp 2>&1 | tail -1)
		[[ $got == *$EXPECTED* ]] || say "INSTALLED $lane COMPILER RED: hello got ${got:0:160}"
	done
}
install_compilers && say "compilers installed beside $(readlink -f "$MAIN_WARP")" || say "compiler install FAILED"

git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"

# the slow tiers in a tree of their own, so the next batch can start in this one at once
[ -d "$SLOW_TREE" ] || cowtree add --detach "$SLOW_TREE" "$tested" >/dev/null 2>&1
git -C "$SLOW_TREE" checkout -q --detach "$tested" || { say "no slow-tier tree at $SLOW_TREE"; exit 1; }
slow=$tree/data/integrate/slow-${tested:0:10}.log
slow_tiers() {
	"$scripts/rebuild.sh" pure "$SLOW_TREE" &&
		"$scripts/gate.sh" --no-rebuild --tiers 1-2 --base "$base" --tag-good "$SLOW_TREE" "$MAIN_WARP"
}
slow_tiers >"$slow" 2>&1 </dev/null &
disown
"$scripts/promote-self.sh" --background "$SLOW_TREE" >/dev/null
say "slow tiers running in the background: $slow"
