#!/usr/bin/env bash
# scripts/integrate/after-push.sh [--if-moved] [<commit>, default origin/main]: keeps the compilers beside ~/dev/bin/warp
# in step with main (user: "main should be immediately synchronized with the binary"). The pusher runs it in the
# background right after its push (notes/agents/common.md "Done"); a cron runs it with --if-moved every 2 minutes as the
# safety net, which returns at once when main is the tip handled last. In its own tree at main's tip: the compilers of
# its lib (scripts/rebuild.sh both: the pusher's smoke build of the same inputs, in the store, is reused), hello through
# them, both moved beside the binary, the user's checkout pulled; then promote-self.sh and tiers 1-2 (gate.sh) of what
# changed since the tip installed before. What goes red becomes a card in Now naming the commits pushed since (a red
# smoke test: urgent). One run at a time (a lock): a second one returns, and the running one loops while main moved
# past what it handled, installing the newest tip first and leaving the slow tiers to the newest.
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
TREE=/Volumes/Worktrees/integrate2
EXPECTED=Hello
HARNESS_PATHS=(src crates Cargo.toml Cargo.lock build.rs)

if_moved=
[ "${1-}" = --if-moved ] && { if_moved=1; shift; }
scripts=$(cd "$(dirname "$0")/.." && pwd)
state=$TREE/data/integrate
say() { echo "after-push: $(date '+%H:%M:%S') $*"; }
main_tip() { git -C "$TREE" ls-remote origin refs/heads/main | cut -f1; }
handled() { cat "$state/handled" 2>/dev/null; }

[ -d "$TREE" ] || cowtree add --detach "$TREE" origin/main >/dev/null || exit 1
mkdir -p "$state"
[ -n "$if_moved" ] && [ "$(main_tip)" = "$(handled)" ] && exit 0
mkdir "$state/lock" 2>/dev/null || { say "a run is going ($state/lock): it takes the newest tip when it ends"; exit 0; }
trap 'rmdir "$state/lock"' EXIT

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
	else
		rm -f "$folder/warp-self.wasm"  # an older pair's: `warp self` says it is missing instead of running it
	fi
	for lane in pure self; do
		[ $lane = self ] && [ ! -f "$folder/warp-self.wasm" ] && continue
		got=$(cd "$MAIN_CHECKOUT" && WARP_NO_WINDOW=1 timeout 300 "$MAIN_WARP" $lane samples/hello/hello.warp 2>&1 | tail -1)
		[[ $got == *$EXPECTED* ]] || red "installed $lane compiler: hello got ${got:0:120}"
	done
	say "compilers of ${tip:0:10} installed beside $MAIN_WARP"
}

# watch <commit>: install its compilers, then (unless main moved on meanwhile) the slow tiers
watch() {
	tip=$1
	since=$(cat "$state/watched" 2>/dev/null || git -C "$TREE" rev-parse "$tip^")
	git -C "$TREE" checkout -q --detach "$tip" || return 1
	pushed=$(git -C "$TREE" log --first-parent --format=%s "$since..$tip" | head -5 | paste -sd ';' -)
	echo "$tip" >"$state/handled"
	"$scripts/rebuild.sh" both "$TREE" || { red "rebuild pure failed" --urgent; return 1; }
	got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { red "smoke test: pure hello got ${got:0:120}" --urgent; return 1; }
	install_compilers
	git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"
	git -C "$TREE" diff --quiet "$since" "$tip" -- "${HARNESS_PATHS[@]}" ||
		say "HARNESS CHANGED: cargo build --release of ${tip:0:10}, installed apart (notes/agents/integrator.md)"
	echo "$tip" >"$state/watched"
	[ "$(main_tip)" = "$tip" ] || return 0

	promoted=$("$scripts/promote-self.sh" "$TREE" | tail -1)
	echo "$promoted"
	[[ $promoted == *"NO FIXPOINT"* || $promoted == *FAILED* ]] && red "self lane: ${promoted:0:160}" --urgent
	install_compilers
	"$scripts/gate.sh" --no-rebuild --tiers 1-2 --base "$since" --tag-good "$TREE" "$MAIN_WARP" >"$state/gate-${tip:0:10}.log" 2>&1 ||
		red "tiers 1-2: $(grep -a '^FAIL' "$state/gate-${tip:0:10}.log" | head -3 | paste -sd ';' -)"
	say "watched ${tip:0:10}"
}

git -C "$TREE" fetch -q origin || exit 1
watch "$(git -C "$TREE" rev-parse "${1:-origin/main}")"
[ $# -gt 0 ] && exit 0
while newest=$(main_tip) && [ "$newest" != "$(handled)" ]; do
	git -C "$TREE" fetch -q origin && watch "$newest"
done
