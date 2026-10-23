#!/usr/bin/env bash
# scripts/rebuild.sh [--check] [--force] [pure|self|both] [<worktree>]: the compiler pair of a worktree's lib/ in its data/
# without building it when it can (notes/compiler_rebuilds.md). Run only by hand or by self-check.sh --rebuild: nothing
# calls it on a change of lib/, and no warp binary rebuilds a compiler by itself.
#   1. data/<compiler>.wasm whose data/<compiler>.wasm.inputs names the current inputs hash (compiler-inputs.sh): kept
#   2. else the shared store's pair of that hash, copied: /workspace/cow/compilers/<hash>/ on the box,
#      /Volumes/Build/compilers/<hash>/ on the Mac (WARP_COMPILER_STORE names another)
#   3. else, unless --check, `warp rebuild` in a heavy slot (scripts/slot.sh on the Mac, /workspace/box-slot.sh on the
#      box), the result published to the store; the store's `main` link moves to it only for a build of origin/main
# --check: steps 1 and 2 only, exit 3 on a miss. --force: step 3 whatever the hash says.
# The warp binary building: WARP_BINARY, else the worktree's target/debug/warp, else main's. Its Rust fingerprint:
# WARP_RUST_FINGERPRINT, else the warp.build section of the compiler.wasm beside it (the pair that binary built).
# warp-host takes the compiler beside it first, then lib/../data, then <prefix>/share/warp (card pure-c-imports): so the
# pair, once in data/, is also copied beside the tree's warp-host (target/{debug,host,release}/), where an older one
# would shadow it. Not beside a lone warp binary: warp-rust judges its own target/debug/ build by the path baked in it. The lookup itself stays warp-host's: this only fills its first folder.
# Every hit and miss is a line of <store>/rebuilds.log: time, tree, hash, compiler, hit or miss.
set -uo pipefail
check= force= which=both tree=
for word in "$@"; do
	case $word in
		--check) check=1 ;;
		--force) force=1 ;;
		pure | self | both) which=$word ;;
		*) tree=$word ;;
	esac
done
tree=$(cd "${tree:-$(git rev-parse --show-toplevel)}" && pwd) || exit 1
scripts=$(cd "$(dirname "$0")" && pwd)
lib=$tree/lib data=$tree/data
if [ "$(uname)" = Darwin ]; then
	store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers} main_warp=/Users/me/dev/bin/warp-rust
	[ -x "$main_warp" ] || main_warp=/Users/me/dev/bin/warp
	slot=("$scripts/slot.sh")
else
	store=${WARP_COMPILER_STORE:-/workspace/cow/compilers} main_warp=/workspace/cow/warp/target/debug/warp
	slot=(/workspace/box-slot.sh)
fi
[ -n "${WARP_SLOT-}" ] && slot=()  # inside a slot already (self-check.sh)
warp=${WARP_BINARY:-}
[ -n "$warp" ] || { [ -x "$tree/target/debug/warp" ] && warp=$tree/target/debug/warp; } || warp=$main_warp
[ -x "$warp" ] || { echo "rebuild.sh: no warp binary at $warp (WARP_BINARY names one)" >&2; exit 1; }

# the third line of a build file's warp.build section: WARP_SOURCE of the binary that built it
build_part() { LC_ALL=C tr '\0' '\n' <"$1" 2>/dev/null | LC_ALL=C sed -n '/warp\.build/{n;n;p;q;}' | LC_ALL=C grep -ao '^[0-9a-f]\{16\}' | head -1; }
rust_fingerprint() {
	[ -n "${WARP_RUST_FINGERPRINT-}" ] && { echo "$WARP_RUST_FINGERPRINT"; return; }
	local beside
	beside=$(dirname "$(readlink -f "$warp" 2>/dev/null || echo "$warp")")/compiler.wasm
	[ -f "$beside" ] && build_part "$beside"
}
rust=$(rust_fingerprint)
[ -n "$rust" ] || echo "rebuild.sh: no Rust fingerprint for $warp (no compiler.wasm beside it): WARP_RUST_FINGERPRINT names it" >&2
hash=$("$scripts/compiler-inputs.sh" "$lib" "$rust") || exit 1
mkdir -p "$store" "$data" 2>/dev/null
logged() { echo "$(date '+%Y-%m-%d %H:%M:%S %Z') tree=$tree hash=$hash $1 $2" >>"$store/rebuilds.log" 2>/dev/null; echo "rebuild.sh: $1 $2 ($hash)"; }
copied() { cp -c "$1" "$2" 2>/dev/null || cp --reflink=auto "$1" "$2" 2>/dev/null || cp "$1" "$2"; }
inputs_line() { echo "$hash rust=$rust commit=$(git -C "$tree" rev-parse --short HEAD 2>/dev/null) built=$(date '+%Y-%m-%dT%H:%M:%S%z')"; }

# a file into the store under its hash, never over one there (write-once)
publish() {
	local file=$1 name
	name=$(basename "$file")
	mkdir -p "$store/$hash"
	[ -f "$store/$hash/$name" ] && return
	copied "$file" "$store/$hash/$name.new-$$" && mv -n "$store/$hash/$name.new-$$" "$store/$hash/$name"
	rm -f "$store/$hash/$name.new-$$"
	inputs_line >"$store/$hash/$name.inputs"
}
# the store's main link to this hash, when the tree is origin/main and its lib/compiler unchanged
mark_main() {
	local head remote
	head=$(git -C "$tree" rev-parse HEAD 2>/dev/null) || return
	remote=$(git -C "$tree" ls-remote origin refs/heads/main 2>/dev/null | cut -f1)
	[ -n "$remote" ] && [ "$head" = "$remote" ] && git -C "$tree" diff --quiet HEAD -- lib/compiler && ln -sfn "$hash" "$store/main"
}

# data/'s file beside each of the tree's warp-host binaries, unless the same file is there
besides() {
	local name=$1 folder
	for folder in "$tree"/target/debug "$tree"/target/host "$tree"/target/release; do
		[ -x "$folder/warp-host" ] || continue
		cmp -s "$data/$name.wasm" "$folder/$name.wasm" && continue
		copied "$data/$name.wasm" "$folder/$name.wasm.new-$$" && mv "$folder/$name.wasm.new-$$" "$folder/$name.wasm" &&
			cp "$data/$name.wasm.inputs" "$folder/$name.wasm.inputs" && echo "rebuild.sh: $name beside ${folder#"$tree"/}"
	done
}
one() {
	local name=$1 file=$data/$1.wasm built status
	if [ -z "$force" ] && [ -f "$file" ] && [ "$(cut -d' ' -f1 "$file.inputs" 2>/dev/null)" = "$hash" ]; then logged "$name" hit-data; return 0; fi
	if [ -z "$force" ] && [ -f "$store/$hash/$name.wasm" ]; then
		copied "$store/$hash/$name.wasm" "$file.new-$$" && mv "$file.new-$$" "$file" && inputs_line >"$file.inputs"
		logged "$name" hit-store
		return 0
	fi
	if [ -n "$check" ]; then logged "$name" miss; return 3; fi
	logged "$name" miss-building
	local lane=pure
	[ "$name" = compiler-self ] && lane=self
	built=$(cd "$tree" && WARP_LIB=$lib WARP_INPUTS_HASH=$hash "${slot[@]}" "$warp" rebuild $lane 2>&1)
	status=$?
	[ $status = 0 ] || { echo "$built" | tail -5 >&2; logged "$name" build-failed; return 1; }
	local path
	path=$(echo "$built" | sed -n 's/^built //p' | tail -1)
	[ -n "$path" ] && [ "$path" != "$file" ] && [ -f "$path" ] && copied "$path" "$file"
	[ -f "$file" ] || { echo "rebuild.sh: warp rebuild said no file ($built)" >&2; return 1; }
	inputs_line >"$file.inputs"
	publish "$file"
	logged "$name" built
}

result=0
case $which in
	pure) one compiler || result=$? ;;
	self) one compiler-self || result=$? ;;
	both) one compiler || result=$?; [ $result = 1 ] || one compiler-self || result=$? ;;
esac
if [ $result = 0 ]; then
	[ $which = self ] || besides compiler
	[ $which = pure ] || besides compiler-self
fi
[ $result = 0 ] && [ -f "$data/compiler.wasm" ] && [ -f "$data/compiler-self.wasm" ] && [ -f "$store/$hash/compiler-self.wasm" ] && mark_main
exit $result
