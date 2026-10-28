#!/usr/bin/env bash
# scripts/integrate/after-push.sh [--if-moved] [<commit>, default origin/main]: keeps the compilers beside ~/dev/bin/warp
# in step with main (user: "main should be immediately synchronized with the binary"). The pusher runs it in the
# background right after its push (notes/agents/common.md "Done"); a cron runs it with --if-moved every 2 minutes as the
# safety net, which returns at once when main is the tip handled last. In its own tree at main's tip: the compilers of
# its lib (scripts/rebuild.sh both: the pusher's smoke build of the same inputs, in the store, is reused), hello through
# them, both moved beside the binary, the user's checkout pulled; then promote-self.sh and tiers 1-2 (gate.sh) of what
# changed since the tip installed before. What goes red becomes a card in Now naming the commits pushed since (a red
# smoke test: urgent). One run at a time (a lock): a second one returns, and the running one loops while main moved
# past what it handled, installing the newest tip first. The slow tiers never hold an install: `--slow` runs them in a
# tree of their own (started by each install, which stops the one still running on an older tip), and installs the
# warp-self.wasm it promotes when the installed warp.wasm is still of those inputs.
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp
MAIN_CHECKOUT=/Users/me/dev/angles/warp
TREE=/Volumes/Worktrees/integrate2
SLOW_TREE=/Volumes/Worktrees/integrate-slow
EXPECTED=Hello
HARNESS_PATHS=(src crates Cargo.toml Cargo.lock build.rs)
SEED=$(dirname "$MAIN_WARP")/warp-seed.wasm  # the seed refresh-seed.sh installs beside the shared binary
BUILDS=/Users/me/dev/bin/warp-builds  # one folder: the installed harness and its compilers, no other builds or links
# a target dir of its own: the shared one (~/.cargo/config.toml) is overwritten by other sessions' branch builds
CARGO_TARGET=/Volumes/Build/target-integrate

if_moved= slow=
arguments=("$@")
script_sum=$(shasum <"$0")
[ "${1-}" = --if-moved ] && { if_moved=1; shift; }
[ "${1-}" = --slow ] && { slow=1; shift; }
scripts=$(cd "$(dirname "$0")/.." && pwd)
cd "$MAIN_CHECKOUT" || exit 1  # todo finds the board through the checkout's repo (the cron starts in ~)
installed_state=$TREE/data/integrate
state=$installed_state
[ -n "$slow" ] && { TREE=$SLOW_TREE; state=$SLOW_TREE/data/integrate; }
say() { echo "after-push: $(date '+%H:%M:%S') $*"; }
main_tip() { git -C "$TREE" ls-remote origin refs/heads/main | cut -f1; }
handled() { cat "$state/handled" 2>/dev/null; }

[ -d "$TREE" ] || cowtree add --detach "$TREE" origin/main >/dev/null || exit 1
mkdir -p "$state"
[ -n "$if_moved" ] && [ "$(main_tip)" = "$(handled)" ] && exit 0
kill_tree() { # kill_tree <pid>: the process and all it started, the parent first (it would act on a killed child's failure)
	local children child
	children=$(pgrep -P "$1")
	kill "$1" 2>/dev/null
	for child in $children; do kill_tree "$child"; done
}
# the lock holds the runner's pid: a dead runner's (a reboot, a kill) is taken over. The install lane's runner takes the
# newest tip when it ends; the slow lane's is stopped, so the slow tiers run on the newest install, never queued behind
lock=$state/lock
if ! mkdir "$lock" 2>/dev/null; then
	holder=$(cat "$lock/pid" 2>/dev/null)
	if [ -n "$holder" ] && kill -0 "$holder" 2>/dev/null; then
		[ -z "$slow" ] && { say "run $holder is going: it takes the newest tip when it ends"; exit 0; }
		say "slow tiers of run $holder stopped for a newer install"
		kill_tree "$holder"
		while kill -0 "$holder" 2>/dev/null; do sleep 1; done
	fi
	rm -rf "$lock" && mkdir "$lock" 2>/dev/null || exit 0
fi
echo $$ >"$lock/pid"
trap 'rm -rf "$lock"' EXIT
# a loop running on goes on with the functions it read at its start: a changed script starts over (exec skips the trap)
rerun_if_changed() {
	[ "$(shasum <"$0")" = "$script_sum" ] && return
	say "the script changed: it starts over"
	rm -rf "$lock"
	exec "$0" "${arguments[@]}"
}

red() { # red <what: detail> [--urgent]: one card per kind of failure (its key from <what>), noted again per failing tip
	local key text="main ${tip:0:10}: $1 (pushed since ${since:0:10}: $pushed)"
	key=$(echo "${1%%:*}" | tr 'A-Z ' 'a-z-' | tr -cd 'a-z0-9-')
	if todo show "$key" 2>/dev/null | head -1 | grep -qv '^\[Done\]\|Archived'; then
		todo note "$key" "$text" >/dev/null
	else
		todo add Now "$text" --key "$key" "${@:2}" >/dev/null
	fi
	say "RED $1 (card $key)"
}

# copy_compilers <lib> <folder>: the compilers a lib/ keeps (runtime.wasm, which every compile links, and warp.wasm;
# warp-self.wasm only when promoted from the same inputs; modules/*.wasm) into the folder with their machine code (.cwasm) and kept values (.kept/), each file moved into place
# whole, its machine code before it: a run that finds a compiler finds its code too (card atomic-install)
copy_compilers() {
	local from=$1 folder=$2 inputs name part
	inputs=$(cut -d' ' -f1 "$from/warp.wasm.inputs")
	for name in runtime.wasm warp.wasm warp-self.wasm; do
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
	# the prebuilt standard modules programs link to (a module missing or older than its text is compiled from source)
	mkdir -p "$folder/modules"
	for part in "$from"/modules/*.wasm; do
		[ -f "$part" ] || continue
		name=modules/$(basename "$part")
		cp "$part" "$folder/$name.new" && mv "$folder/$name.new" "$folder/$name" || return 1
	done
}

# the tree's compilers beside the shared binary
install_compilers() {
	[ -n "$slow" ] && [ "$(cat "$installed_state/watched")" != "$tip" ] && return  # a newer tip is installed meanwhile
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
	local built folder
	built=$("$MAIN_WARP" version 2>&1 | sed -n 's/^release build \([0-9a-f]*\).*/\1/p')
	folder=$(dirname "$(readlink -f "$MAIN_WARP")")
	# an install by an older script lacks what the current one brings
	[ -x "$folder/warp-runtime" ] && [ -d "$folder/modules" ] && [ -n "$built" ] && git -C "$TREE" diff --quiet "$built" "$tip" -- "${HARNESS_PATHS[@]}" 2>/dev/null
}

# built_from_tip <folder>: cargo build of $TREE copied there, true when the binary names $tip, on main or detached
built_from_tip() {
	(cd "$TREE" && CARGO_TARGET_DIR=$CARGO_TARGET cargo build --release -q) || { red "harness: cargo build --release failed" --urgent; return 1; }
	cp "$CARGO_TARGET/release/warp" "$1/warp" || return 1
	# the stub `warp build` appends a program's machine code to, built alone: with warp's features unified it is 6.9 MB,
	# not 1.3 MB (card executable-prints-value)
	(cd "$TREE" && CARGO_TARGET_DIR=$CARGO_TARGET cargo build --release -q -p warp-runtime --bin warp-runtime) &&
		cp "$CARGO_TARGET/release/warp-runtime" "$1/warp-runtime" || red "harness: the runtime stub warp-runtime did not build"
	local said
	said=$("$1/warp" version 2>&1)  # read whole: grep -q quitting early would fail the pipe (pipefail)
	grep -Eq "^release build ${tip:0:9}( on main)? " <<<"$said"
}

# the harness of $tip built and installed: its folder in $BUILDS with warp.wasm built by it, hello, the compilers
# copied in, then ~/dev/bin/warp links there by one rename (a run never meets the new binary without its compilers:
# card atomic-install) and every older folder goes (user: no stale builds, no warp-<hash> links)
install_harness() {
	local folder=$BUILDS/${tip:0:9} got built
	say "harness of ${tip:0:10} changed: cargo build --release"
	mkdir -p "$folder" || return 1
	built_from_tip "$folder" || built_from_tip "$folder" ||
		{ red "harness: the build says $("$folder/warp" version 2>&1 | sed -n 2p), not ${tip:0:9} of main" --urgent; rm -rf "$folder"; return 1; }
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
	# warp's wasmtime C API beside warp and in the user's lib/, where native builds link it from (card wasmtime-bundled)
	"$scripts/wasmtime-c-api.sh" "$(dirname "$(readlink -f "$MAIN_WARP")")" "$MAIN_CHECKOUT/lib" ||
		red "wasmtime C API: scripts/wasmtime-c-api.sh failed" --urgent
	"$scripts/rebuild.sh" both "$TREE" || { red "rebuild pure failed" --urgent; return 1; }
	got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { red "smoke test: pure hello got ${got:0:120}" --urgent; return 1; }
	install_compilers
	git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"
	echo "$tip" >"$state/watched"
	"$0" --slow </dev/null &
	disown
}

# slow_tiers <commit>: promote-self.sh and tiers 1-2 of what changed since the last commit they ran on
slow_tiers() {
	tip=$1
	since=$(cat "$state/tested" 2>/dev/null || git -C "$TREE" rev-parse "$tip^")
	pushed=$(git -C "$TREE" log --first-parent --format=%s "$since..$tip" | head -5 | paste -sd ';' -)
	git -C "$TREE" checkout -q --detach "$tip" || return 1
	"$scripts/rebuild.sh" both "$TREE" >/dev/null || return 1
	promoted=$("$scripts/promote-self.sh" "$TREE" | tail -1)
	echo "$promoted"
	[[ $promoted == *"NO FIXPOINT"* || $promoted == *FAILED* ]] && red "self lane: ${promoted:0:160}" --urgent
	install_compilers
	"$scripts/gate.sh" --no-rebuild --tiers 1-2 --base "$since" --tag-good "$TREE" "$MAIN_WARP" >"$state/gate-${tip:0:10}.log" 2>&1 ||
		red "tiers 1-2: $(grep -a '^FAIL' "$state/gate-${tip:0:10}.log" | head -3 | paste -sd ';' -)"
	echo "$tip" >"$state/tested"
	say "slow tiers of ${tip:0:10} done"
}

if [ -n "$slow" ]; then
	git -C "$TREE" fetch -q origin || exit 1
	installed=$(cat "$installed_state/watched")
	[ "$installed" = "$(cat "$state/tested" 2>/dev/null)" ] || slow_tiers "$installed"
	exit 0
fi

git -C "$TREE" fetch -q origin || exit 1
watch "$(git -C "$TREE" rev-parse "${1:-origin/main}")"
[ $# -gt 0 ] && exit 0
while newest=$(main_tip) && [ "$newest" != "$(handled)" ]; do
	rerun_if_changed
	git -C "$TREE" fetch -q origin && watch "$newest"
done
