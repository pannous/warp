#!/usr/bin/env bash
# scripts/integrate/after-push.sh [--if-moved] [<commit>, default origin/main]: keeps the compilers beside ~/dev/bin/warp
# in step with main (user: "main should be immediately synchronized with the binary"). The pusher runs it in the
# background right after its push (notes/agents/common.md "Done"); a cron runs it with --if-moved every 2 minutes as the
# safety net, which returns at once when main is the tip handled last. In its own tree at main's tip: the compilers of
# its lib (scripts/rebuild.sh both: the pusher's smoke build of the same inputs, in the store, is reused), hello through
# them, both moved beside the binary (in two stages when they must be built: the native warp first, with the last
# install's compilers, then the compilers beside it), the user's checkout pulled; then promote-self.sh and tiers 1-2 (gate.sh) of what
# changed since the tip installed before. What goes red becomes a card in Now naming the commits pushed since (a red
# smoke test: urgent). One run at a time (a lock): a second one returns, and the running one loops while main moved
# past what it handled, installing the newest tip first. The slow tiers never hold an install: `--slow` runs them in a
# tree of their own (started by each install, which stops the one still running on an older tip), and installs the
# warp-self.wasm it promotes when the installed warp.wasm is still of those inputs.
set -uo pipefail
MAIN_WARP=/Users/me/dev/bin/warp  # the native warp (core/native/warp.warp), in a folder of its own (install_native)
# the native warp's folder by a name that outlives installs: what executables it builds find libwarp_wasmtime by
# (their rpath, card native-rpath-builds-folder)
MAIN_LIB=/Users/me/dev/bin/lib
MAIN_CHECKOUT=/Users/me/dev/angles/warp
TREE=/Volumes/Worktrees/integrate2
SLOW_TREE=/Volumes/Worktrees/integrate-slow
EXPECTED=Hello
SEED=$(dirname "$MAIN_WARP")/warp-seed.wasm  # the seed refresh-seed.sh installs beside the shared binary
BUILDS=/Users/me/dev/bin/warp-builds  # one folder: the installed native warp with its lib/, no other builds or links

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

# the tree's compilers into the installed native warp's folder (the slow lane's promoted warp-self.wasm)
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

# linked <file> <link>: the link (~/dev/bin/warp, lib) made to point to the file or folder by one rename,
# relative to the link's folder (mv -h: a link to a folder is replaced, not moved into)
linked() {
	ln -sfn "${1#"$(dirname "$2")/"}" "$2.new" && mv -fh "$2.new" "$2"
}

# the build folders ~/dev/bin/warp doesn't link into removed, only once the links are in place: a link
# never dangles (2026-10-28: warp pointed into a folder the Rust install had pruned). The newest KEPT_BUILDS folders stay
# too, and any folder a process still has open: a run started on the previous build reads its warp.wasm and modules
# until it ends (a full suite wrote an empty results.txt, "No module at …/native-fb9bda25b/warp.wasm", 2026-10-29)
KEPT_BUILDS=3
pruned() {
	local kept=() link folder
	for link in "$MAIN_WARP"; do kept+=("$(basename "$(dirname "$(readlink "$link")")")"); done
	for folder in $(ls -dt "$BUILDS"/*/ | tail -n +$((KEPT_BUILDS + 1))); do
		folder=${folder%/}
		[[ " ${kept[*]} " == *" $(basename "$folder") "* ]] && continue
		lsof +D "$folder" >/dev/null 2>&1 && continue
		rm -rf "$folder"
	done
}

# the native warp of $tip in a folder of its own, as a release lays it out (decision rust-gone-in-1-5): the tree's lib/
# with its compilers and libwarp_wasmtime, and the native warp built from it (core/native/warp.warp by the installed
# native warp's `warp build native`, the warp-from-warp chain); hello by it, then ~/dev/bin/warp links there by one rename and the older native folders go.
# The seed stays beside ~/dev/bin/warp, where the native `warp rebuild` looks (the link's folder)
NATIVE_PREFIX=native-
# native_built <folder> <builder>: the native warp built from the folder's core/native/warp.warp into <folder>/warp
native_built() {
	(cd "$1/compiler/native" && WARP_NO_WINDOW=1 WARP_LIB=$1 "$2" build native warp.warp >/dev/null 2>&1) &&
		mv "$1/compiler/native/warp" "$1/warp"
}
# install_native [early]: early, before the tip's compilers are built, with the tree's compilers of the last install
# (stage one: a driver fix reaches the user in minutes, never hours behind a compiler build; user 2026-10-30). Its
# failure is only said: stage two installs the native warp again with the tip's compilers
# the tip an ancestor of the installed native warp's (its folder native-<commit>[.anything], an install by hand too):
# a link to it would take the user back (2026-10-30: an install undid the user's newer .express one)
older_than_installed() {
	local installed
	installed=$(basename "$(dirname "$(readlink "$MAIN_WARP")")" | sed "s/^$NATIVE_PREFIX//; s/\..*//")
	[ -n "$installed" ] && ! git -C "$TREE" merge-base --is-ancestor "$installed" "$tip" 2>/dev/null &&
		git -C "$TREE" merge-base --is-ancestor "$tip" "$installed" 2>/dev/null
}
early_failed() { say "stage one: $1; the native warp follows with its compilers"; }
install_native() {
	local folder=$BUILDS/$NATIVE_PREFIX${tip:0:9} got made failed=(red) early=${1-}
	[ -n "$early" ] && failed=(early_failed)
	rm -rf "$folder.new" && mkdir -p "$folder.new" || return 1
	rsync -aL --exclude /data "$TREE/lib/" "$folder.new/" || { "${failed[@]}" "native warp: lib/ did not copy" --urgent; return 1; }
	native_built "$folder.new" "$MAIN_WARP" ||
		{ "${failed[@]}" "native warp: warp build native core/native/warp.warp failed" --urgent; rm -rf "$folder.new"; return 1; }
	rm -rf "$folder" && mv "$folder.new" "$folder" || return 1
	got=$(cd "$MAIN_CHECKOUT" && WARP_NO_WINDOW=1 timeout 300 "$folder/warp" samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { "${failed[@]}" "native warp: hello got ${got:0:120}" --urgent; rm -rf "$folder"; return 1; }
	made=$("$scripts/prebuilt-parts.sh" "$folder" 2>&1) || { "${failed[@]}" "native warp: ${made:0:200}" --urgent; rm -rf "$folder"; return 1; }
	older_than_installed && { say "native warp of ${tip:0:10} not linked: the installed one is newer"; return 2; }
	linked "$folder/warp" "$MAIN_WARP" && linked "$folder" "$MAIN_LIB" || return 1
	pruned
	say "native warp of ${tip:0:10} installed${early:+ (its compilers follow)}: $MAIN_WARP -> $folder/warp"
}

# watch <commit>: install its compilers, then (unless main moved on meanwhile) the slow tiers
watch() {
	tip=$1
	since=$(cat "$state/watched" 2>/dev/null || git -C "$TREE" rev-parse "$tip^")
	git -C "$TREE" checkout -q --detach "$tip" || return 1
	pushed=$(git -C "$TREE" log --first-parent --format=%s "$since..$tip" | head -5 | paste -sd ';' -)
	echo "$tip" >"$state/handled"
	# warp's wasmtime C API in the user's lib/ too, where native builds link it from (card wasmtime-bundled); the tree's
	# lib/ gets it from rebuild.sh
	"$scripts/wasmtime-c-api.sh" "$MAIN_CHECKOUT/lib" ||
		red "wasmtime C API: scripts/wasmtime-c-api.sh failed" --urgent
	# stage one: the native warp at once, when the tip's compilers are not in the store yet
	local early=
	"$scripts/rebuild.sh" --check pure "$TREE" >/dev/null || { install_native early && early=1; }
	"$scripts/rebuild.sh" both "$TREE" || { red "rebuild pure failed" --urgent; return 1; }
	got=$(cd "$TREE" && WARP_NO_WINDOW=1 WARP_LIB=$TREE/lib timeout 300 "$MAIN_WARP" pure samples/hello/hello.warp 2>&1 | tail -1)
	[[ $got == *$EXPECTED* ]] || { red "smoke test: pure hello got ${got:0:120}" --urgent; return 1; }
	# stage two: its compilers beside it, else the whole native warp
	if [ -n "$early" ]; then install_compilers; else install_native; fi
	git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash || say "the user's checkout did not pull: look at it"
	echo "$tip" >"$state/watched"
	"$0" --slow </dev/null &
	disown
}

# the samples that run, the gap to 2.0 (decision night-samples-modules): test/samples.txt committed on main when it
# changed, so its git log is the history and git diff names the samples that changed (user 2026-10-29); at most once
# every SAMPLES_HOURS, never on every push (the sweep with the rest loaded the machine to 109, 2026-10-30)
SAMPLES_HOURS=4
samples_committed() {
	local count report=test/samples.txt last=$state/samples-at
	[ -f "$last" ] && [ $(($(date +%s) - $(cat "$last"))) -lt $((SAMPLES_HOURS * 3600)) ] && return
	date +%s >"$last"
	count=$("$scripts/samples.sh" "$TREE" | head -1 | sed -n 's/^samples: \([0-9]* of [0-9]*\) run.*/\1/p')
	cp "$TREE/$report" "$MAIN_CHECKOUT/$report" || return
	# the slow tree stays clean for its next checkout
	git -C "$TREE" checkout -q HEAD -- "$report" 2>/dev/null || rm -f "$TREE/$report"
	git -C "$MAIN_CHECKOUT" add -- "$report" && git -C "$MAIN_CHECKOUT" diff --cached --quiet -- "$report" && return
	git -C "$MAIN_CHECKOUT" commit -q -m "chore: samples $count at ${1:0:10}" -- "$report" &&
		git -C "$MAIN_CHECKOUT" pull -q --rebase --autostash && git -C "$MAIN_CHECKOUT" push -q ||
		say "samples of ${1:0:10} not committed: look at the user's checkout"
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
	samples_committed "$tip"
	echo "$tip" >"$state/tested"
	say "slow tiers of ${tip:0:10} done"
}

# the slow tiers paused until the time in this file (seconds since 1970), for someone's quiet CPU (a profile, a timing)
SLOW_PAUSE=$HOME/.cache/warp/slow-tiers-paused-until
if [ -n "$slow" ]; then
	[ "$(cat "$SLOW_PAUSE" 2>/dev/null || echo 0)" -gt "$(date +%s)" ] && { say "slow tiers paused until $(date -r "$(cat "$SLOW_PAUSE")" +%H:%M)"; exit 0; }
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
