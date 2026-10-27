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
SEED=$(dirname "$MAIN_WARP")/warp-seed.wasm  # the seed refresh-seed.sh installs beside the shared binary
BUILDS=/Users/me/dev/bin/main-builds  # one folder: the installed harness and its compilers, no other builds or links
CARGO_BINARY=/Volumes/Build/target/release/warp  # ~/.cargo/config.toml's target-dir

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

# copy_compilers <lib> <folder>: the compilers a lib/ keeps (warp.wasm; warp-self.wasm only when promoted from the
# same inputs) into the folder with their machine code (.cwasm) and kept values (.kept/), each file moved into place
# whole, its machine code before it: a run that finds a compiler finds its code too (card atomic-install)
copy_compilers() {
	local from=$1 folder=$2 inputs name part
	inputs=$(cut -d' ' -f1 "$from/warp.wasm.inputs")
	for name in warp.wasm warp-self.wasm; do
		if [ $name = warp-self.wasm ] && [ "$(cut -d' ' -f1 "$from/$name.inputs" 2>/dev/null)" != "$inputs" ]; then
			rm -f "$folder/$name" "$folder/$name.cwasm"  # an older pair's: `warp self` says it is missing instead of running it
			continue
		fi
		[ -d "$from/$name.kept" ] && mkdir -p "$folder/$name.kept" && cp -R "$from/$name.kept/." "$folder/$name.kept/"
		for part in "$name.cwasm" "$name"; do
			[ -f "$from/$part" ] || continue
			cp "$from/$part" "$folder/$part.new" && mv "$folder/$part.new" "$folder/$part" || return 1
		done
	done
}

# the tree's compilers beside the shared binary
install_compilers() {
	local folder lane got
	folder=$(dirname "$(readlink -f "$MAIN_WARP")")
	copy_compilers "$TREE/lib" "$folder"
	for lane in pure self; do
		[ $lane = self ] && [ ! -f "$folder/warp-self.wasm" ] && continue
		got=$(cd "$MAIN_CHECKOUT" && WARP_NO_WINDOW=1 timeout 300 "$MAIN_WARP" $lane samples/hello/hello.warp 2>&1 | tail -1)
		[[ $got == *$EXPECTED* ]] || red "installed $lane compiler: hello got ${got:0:120}"
	done
	say "compilers of ${tip:0:10} installed beside $MAIN_WARP"
}

# whether the installed binary was built from the harness of $tip ("release build <commit>" in `warp version`)
harness_current() {
	local built
	built=$("$MAIN_WARP" version 2>&1 | sed -n 's/^release build \([0-9a-f]*\).*/\1/p')
	[ -n "$built" ] && git -C "$TREE" diff --quiet "$built" "$tip" -- "${HARNESS_PATHS[@]}" 2>/dev/null
}

# the harness of $tip built and installed: its folder in $BUILDS with warp.wasm built by it, hello, the compilers
# copied in, then ~/dev/bin/warp links there by one rename (a run never meets the new binary without its compilers:
# card atomic-install) and every older folder goes (user: no stale builds, no warp-<hash> links)
install_harness() {
	local folder=$BUILDS/${tip:0:9} got built
	say "harness of ${tip:0:10} changed: cargo build --release"
	(cd "$TREE" && cargo build --release -q) || { red "harness: cargo build --release failed" --urgent; return 1; }
	mkdir -p "$folder" && cp "$CARGO_BINARY" "$folder/warp" || return 1
	# the seed lies beside ~/dev/bin/warp, which the new binary, run from its own folder, does not see
	# through rebuild.sh, which stamps the build with its inputs: the rebuild.sh of watch then keeps it, not building twice
	built=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_BINARY=$folder/warp WARP_SEED=$SEED "$scripts/rebuild.sh" pure "$TREE" 2>&1 | tail -1)
	got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$folder/warp" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { red "harness: hello with the new binary got ${got:0:120} (its rebuild said: ${built:0:120})" --urgent; rm -rf "$folder"; return 1; }
	copy_compilers "$TREE/lib" "$folder" || { red "harness: the compilers did not copy into $folder" --urgent; rm -rf "$folder"; return 1; }
	ln -sfn "${folder#"$(dirname "$MAIN_WARP")/"}/warp" "$MAIN_WARP.new" && mv -f "$MAIN_WARP.new" "$MAIN_WARP" || return 1
	find "$BUILDS" -mindepth 1 -maxdepth 1 ! -name "${tip:0:9}" -exec rm -rf {} +
	say "harness of ${tip:0:10} installed: $MAIN_WARP -> $folder/warp"
}

# watch <commit>: install its compilers, then (unless main moved on meanwhile) the slow tiers
watch() {
	tip=$1
	since=$(cat "$state/watched" 2>/dev/null || git -C "$TREE" rev-parse "$tip^")
	git -C "$TREE" checkout -q --detach "$tip" || return 1
	pushed=$(git -C "$TREE" log --first-parent --format=%s "$since..$tip" | head -5 | paste -sd ';' -)
	echo "$tip" >"$state/handled"
	harness_current || install_harness || return 1
	"$scripts/rebuild.sh" both "$TREE" || { red "rebuild pure failed" --urgent; return 1; }
	got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { red "smoke test: pure hello got ${got:0:120}" --urgent; return 1; }
	install_compilers
	git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"
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
