#!/usr/bin/env bash
# scripts/rebuild.sh [--check] [--force] [pure|self|both] [<worktree>]: the compilers of a worktree's lib/, in that lib/ itself
# without building them when it can (notes/compiler_rebuilds.md). Run only by hand or by self-check.sh --rebuild: nothing
# calls it on a change of lib/, and no warp binary rebuilds a compiler by itself.
# pure (the default): warp.wasm, built by the seed. self: warp-self.wasm, no build of its own: warp.wasm promoted by
# `warp rebuild self` when warp.wasm compiles compiler.warp once more (scripts/promote-self.sh runs that and never fails its caller).
# both: pure, then warp-self.wasm only from lib/ or the store (steps 1-2), never promoted here: the gate needs warp.wasm.
# Transition: a file of the old name (compiler.wasm, compiler-self.wasm) in lib/ or the store counts when the new one
# is missing, and is copied under the new name.
#   1. lib/<compiler>.wasm (warp, warp-self) whose lib/<compiler>.wasm.inputs names the current inputs hash (compiler-inputs.sh): kept
#   2. else the shared store's pair of that hash, copied: /workspace/cow/compilers/<hash>/ on the box,
#      /Volumes/Build/compilers/<hash>/ on the Mac (WARP_COMPILER_STORE names another)
#   3. else, unless --check, `warp rebuild`, the result published to the store; the store's `main` link moves to it only for a build of origin/main
# --check: steps 1 and 2 only, exit 3 on a miss. --force: step 3 whatever the hash says.
# The warp binary building: WARP_BINARY, else the shared one (main's: /workspace/cow/warp-rust-latest on the box). Its Rust fingerprint:
# WARP_RUST_FINGERPRINT, else the warp.build section of the warp.wasm (or compiler.wasm) beside it (what that binary built).
# warp-host takes the compiler beside it first, then lib/ itself, then <prefix>/share/warp (card pure-c-imports): so the
# pair, once in lib/, is also copied beside the tree's warp-host (target/{debug,host,release}/), where an older one
# would shadow it. Not beside a lone warp binary: warp-rust judges its own target/debug/ build by the path baked in it. The lookup itself stays warp-host's: this only fills its first folder.
# Every hit and miss is a line of <store>/rebuilds.log: time, tree, hash, compiler, hit or miss.
set -uo pipefail
check= force= which=pure tree=
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
lib=$tree/lib compilers=$tree/lib
if [ "$(uname)" = Darwin ]; then
	store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers} main_warp=/Users/me/dev/bin/warp-rust
	[ -x "$main_warp" ] || main_warp=/Users/me/dev/bin/warp
else
	store=${WARP_COMPILER_STORE:-/workspace/cow/compilers} main_warp=/workspace/cow/warp-rust-latest
fi
# the shared binary, never the tree's own target/debug/warp: Rust src/ is frozen, and a tree's copy is a stale one
# (boxtree copies target/), or a per-tree cargo build (card scripts-off-cargo)
warp=${WARP_BINARY:-$main_warp}
[ -x "$warp" ] || { echo "rebuild.sh: no warp binary at $warp (WARP_BINARY names one)" >&2; exit 1; }

# the third line of a build file's warp.build section: WARP_SOURCE of the binary that built it
build_part() { LC_ALL=C tr '\0' '\n' <"$1" 2>/dev/null | LC_ALL=C sed -n '/warp\.build/{n;n;p;q;}' | LC_ALL=C grep -ao '^[0-9a-f]\{16\}' | head -1; }
rust_fingerprint() {
	[ -n "${WARP_RUST_FINGERPRINT-}" ] && { echo "$WARP_RUST_FINGERPRINT"; return; }
	local folder
	folder=$(dirname "$(readlink -f "$warp" 2>/dev/null || echo "$warp")")
	if [ -f "$folder/warp.wasm" ]; then build_part "$folder/warp.wasm"; elif [ -f "$folder/compiler.wasm" ]; then build_part "$folder/compiler.wasm"; fi
}
rust=$(rust_fingerprint)
[ -n "$rust" ] || echo "rebuild.sh: no Rust fingerprint for $warp (no warp.wasm beside it): WARP_RUST_FINGERPRINT names it" >&2
# the seed building warp.wasm, named for the build and its inputs hash alike: WARP_SEED, else the one beside the binary
seed_beside() { local folder; for folder in "$(dirname "$warp")" "$(dirname "$(readlink -f "$warp" 2>/dev/null || echo "$warp")")"; do [ -f "$folder/warp-seed.wasm" ] && { echo "$folder/warp-seed.wasm"; return; }; done; }
export WARP_SEED=${WARP_SEED:-$(seed_beside)}
hash=$("$scripts/compiler-inputs.sh" "$lib" "$rust") || exit 1
mkdir -p "$store" "$compilers" 2>/dev/null
logged() { echo "$(date '+%Y-%m-%d %H:%M:%S %Z') tree=$tree hash=$hash $1 $2" >>"$store/rebuilds.log" 2>/dev/null; echo "rebuild.sh: $1 $2 ($hash)"; }
copied() { cp -c "$1" "$2" 2>/dev/null || cp --reflink=auto "$1" "$2" 2>/dev/null || cp "$1" "$2"; }
inputs_line() { echo "$hash rust=$rust commit=$(git -C "$tree" rev-parse --short HEAD 2>/dev/null) built=$(date '+%Y-%m-%dT%H:%M:%S%z')"; }

# a file into the store under its hash, by its name or the path given, never over one there (write-once)
publish() {
	local file=$1 name=${2:-$(basename "$1")}
	mkdir -p "$(dirname "$store/$hash/$name")"
	[ -f "$store/$hash/$name" ] && return
	copied "$file" "$store/$hash/$name.new-$$" && mv -n "$store/$hash/$name.new-$$" "$store/$hash/$name"
	rm -f "$store/$hash/$name.new-$$"
	inputs_line >"$store/$hash/$name.inputs"
}
# the store's main link to this hash, when the tree is origin/main and its core unchanged
mark_main() {
	local head remote
	head=$(git -C "$tree" rev-parse HEAD 2>/dev/null) || return
	remote=$(git -C "$tree" ls-remote origin refs/heads/main 2>/dev/null | cut -f1)
	[ -n "$remote" ] && [ "$head" = "$remote" ] && git -C "$tree" diff --quiet HEAD -- core && ln -sfn "$hash" "$store/main"
}

# lib/'s file beside each of the tree's warp-host binaries, unless the same file is there
besides() {
	local name=$1 folder
	for folder in "$tree"/target/debug "$tree"/target/host "$tree"/target/release; do
		[ -x "$folder/warp-host" ] || continue
		cmp -s "$compilers/$name.wasm" "$folder/$name.wasm" && continue
		copied "$compilers/$name.wasm" "$folder/$name.wasm.new-$$" && mv "$folder/$name.wasm.new-$$" "$folder/$name.wasm" &&
			cp "$compilers/$name.wasm.inputs" "$folder/$name.wasm.inputs" && echo "rebuild.sh: $name beside ${folder#"$tree"/}"
	done
}
# the files `warp rebuild` writes beside a compiler, kept and fetched with it, as found in the folder: runtime.wasm
# beside warp.wasm (card runtime-prebuilt) with its machine code, which linked runs load (card runtime-cwasm-shipped),
# the standard modules in modules/ (card modules-prebuilt)
companions() { [ "$1" = warp ] && (cd "$2" && ls runtime.wasm.cwasm runtime.wasm modules/*.wasm 2>/dev/null); }
# a compiler's name before the rename (transition)
legacy() { case $1 in warp) echo compiler ;; warp-self) echo compiler-self ;; *) echo "$1" ;; esac; }
# one <name> [check]: lib/<name>.wasm for these inputs, from lib/, the store, else built (promoted for warp-self)
one() {
	local name=$1 file=$compilers/$1.wasm built status old
	local check=${2:-$check}
	old=$compilers/$(legacy "$name").wasm
	if [ -z "$force" ] && [ ! -f "$file" ] && [ -f "$old" ] && [ "$(cut -d' ' -f1 "$old.inputs" 2>/dev/null)" = "$hash" ]; then
		copied "$old" "$file" && cp "$old.inputs" "$file.inputs" && logged "$name" hit-lib-old-name
	fi
	if [ -z "$force" ] && [ -f "$file" ] && [ "$(cut -d' ' -f1 "$file.inputs" 2>/dev/null)" = "$hash" ]; then logged "$name" hit-lib; return 0; fi
	local stored=$store/$hash/$name.wasm
	[ -f "$stored" ] || stored=$store/$hash/$(legacy "$name").wasm
	if [ -z "$force" ] && [ -f "$stored" ]; then
		copied "$stored" "$file.new-$$" && mv "$file.new-$$" "$file" && inputs_line >"$file.inputs"
		for companion in $(companions "$name" "$store/$hash"); do mkdir -p "$(dirname "$compilers/$companion")" && copied "$store/$hash/$companion" "$compilers/$companion"; done
		logged "$name" hit-store
		return 0
	fi
	if [ -n "$check" ]; then logged "$name" miss; return 3; fi
	logged "$name" miss-building
	local lane=pure
	[ "$name" = warp-self ] && lane=self
	built=$(cd "$tree" && WARP_LIB=$lib WARP_INPUTS_HASH=$hash "$warp" rebuild $lane 2>&1)
	status=$?
	[ $status = 0 ] || { echo "$built" | tail -5 >&2; logged "$name" build-failed; return 1; }
	local path
	path=$(echo "$built" | sed -n 's/^built //p' | tail -1)
	[ -n "$path" ] && [ "$path" != "$file" ] && [ -f "$path" ] && copied "$path" "$file"
	[ -f "$file" ] || { echo "rebuild.sh: warp rebuild said no file ($built)" >&2; return 1; }
	inputs_line >"$file.inputs"
	publish "$file"
	for companion in $(companions "$name" "$compilers"); do publish "$compilers/$companion" "$companion"; done
	logged "$name" built
}

result=0
self_found=0
case $which in
	pure) one warp || result=$? ;;
	self) one warp-self || result=$?; [ $result = 0 ] && self_found=1 ;;
	# warp-self.wasm only when lib/ or the store has it for these inputs: never promoted here, never a failure
	both) one warp || result=$?; [ $result = 1 ] || { one warp-self check && self_found=1; } ;;
esac
if [ $result = 0 ]; then
	[ $which = self ] || besides warp
	[ $self_found = 1 ] && besides warp-self
fi
[ $result = 0 ] && [ -f "$compilers/warp.wasm" ] && [ -f "$store/$hash/warp.wasm" ] && mark_main
# warp's wasmtime C API in lib/, which `warp build native` links the driver from (card wasmtime-bundled); a no-op when current
"$scripts/wasmtime-c-api.sh" "$lib" || echo "rebuild.sh: warp's wasmtime C API did not build (scripts/wasmtime-c-api.sh): native builds of the driver fail" >&2
exit $result
