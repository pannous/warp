#!/usr/bin/env bash
# scripts/integrate/after-push.sh [<commit>, default origin/main]: the Integrator's watch of main, run after each push
# (workers land their own smoke-tested merges: notes/agents/common.md "Done"; this repeats no merge test). In its own
# tree at that commit: the compilers of its lib (scripts/rebuild.sh both: a build of the same inputs, the pusher's, is
# reused), hello through them, both moved beside ~/dev/bin/warp, the user's checkout pulled; then promote-self.sh and
# tiers 1-2 (gate.sh) of what changed since the last commit watched. What goes red becomes a card in Now naming the
# commits pushed since (a red smoke test: urgent). Long: run it in the background. One run at a time (a lock).
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
TREE=/Volumes/Worktrees/integrate2
EXPECTED=Hello
HARNESS_PATHS=(src crates Cargo.toml Cargo.lock build.rs)

scripts=$(cd "$(dirname "$0")/.." && pwd)
state=$TREE/data/integrate
say() { echo "after-push: $*"; }

[ -d "$TREE" ] || cowtree add --detach "$TREE" origin/main >/dev/null || exit 1
mkdir -p "$state"
mkdir "$state/lock" 2>/dev/null || { say "a run is going ($state/lock): rerun when it ends"; exit 1; }
trap 'rmdir "$state/lock"' EXIT

git -C "$TREE" fetch -q origin || exit 1
tip=$(git -C "$TREE" rev-parse "${1:-origin/main}") || exit 1
since=$(cat "$state/watched" 2>/dev/null || git -C "$TREE" rev-parse "$tip^")
git -C "$TREE" checkout -q --detach "$tip" || exit 1
pushed=$(git -C "$TREE" log --first-parent --format=%s "$since..$tip" | head -5 | paste -sd ';' -)

red() { # red <what> [--urgent]: a card for the pushers, then on
	todo add Now "main ${tip:0:10}: $1 (pushed since ${since:0:10}: $pushed)" "${@:2}" >/dev/null
	say "RED $1"
}

# the tree's compilers beside the shared binary, warp-self.wasm only when promoted from the same inputs
install_compilers() {
	local folder data=$TREE/data inputs lane got
	folder=$(dirname "$(readlink -f "$MAIN_WARP")")
	inputs=$(cut -d' ' -f1 "$data/warp.wasm.inputs")
	cp "$data/warp.wasm" "$folder/warp.wasm.new" && mv "$folder/warp.wasm.new" "$folder/warp.wasm"
	if [ -f "$data/warp-self.wasm" ] && [ "$(cut -d' ' -f1 "$data/warp-self.wasm.inputs" 2>/dev/null)" = "$inputs" ]; then
		cp "$data/warp-self.wasm" "$folder/warp-self.wasm.new" && mv "$folder/warp-self.wasm.new" "$folder/warp-self.wasm"
	fi
	for lane in pure self; do
		[ $lane = self ] && [ ! -f "$folder/warp-self.wasm" ] && continue
		got=$(cd "$MAIN_CHECKOUT" && WARP_NO_WINDOW=1 timeout 300 "$MAIN_WARP" $lane samples/hello/hello.warp 2>&1 | tail -1)
		[[ $got == *$EXPECTED* ]] || red "installed $lane compiler: hello got ${got:0:120}"
	done
	say "compilers of ${tip:0:10} installed beside $MAIN_WARP"
}

"$scripts/rebuild.sh" both "$TREE" || { red "rebuild pure failed" --urgent; exit 1; }
got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
[[ $got == *$EXPECTED* ]] || { red "smoke test: pure hello got ${got:0:120}" --urgent; exit 1; }
install_compilers
git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"
git -C "$TREE" diff --quiet "$since" "$tip" -- "${HARNESS_PATHS[@]}" ||
	say "HARNESS CHANGED: cargo build --release of ${tip:0:10}, installed apart (notes/agents/integrator.md)"
echo "$tip" >"$state/watched"

"$scripts/promote-self.sh" "$TREE" | tail -1
install_compilers
"$scripts/gate.sh" --no-rebuild --tiers 1-2 --base "$since" --tag-good "$TREE" "$MAIN_WARP" >"$state/gate-${tip:0:10}.log" 2>&1 ||
	red "tiers 1-2: $(grep -a '^FAIL' "$state/gate-${tip:0:10}.log" | head -3 | paste -sd ';' -)"
say "watched ${tip:0:10}"
