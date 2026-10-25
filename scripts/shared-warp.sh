#!/usr/bin/env bash
# scripts/shared-warp.sh [<worktree>]: the path of the shared Rust warp, staged with the worktree's compiler pair beside
# it, for the tests that take the pair from beside the binary (tests/host, tests/launcher): no per-tree cargo build
# (card scripts-off-cargo; Rust src/ is frozen, so the shared binary is every tree's).
# The binary: WARP_RUST, else /workspace/cow/warp-rust-latest on the box, ~/dev/bin/warp-rust else ~/dev/bin/warp on the
# Mac (a launcher script there is skipped). The pair: data/'s, which scripts/rebuild.sh --check first fills from the
# shared compiler store for the tree's compiler inputs (never a build), else the store's main pair, said. Both are copied to <worktree>/scratch/shared-warp/,
# so a refresh of the shared binary never changes it under a running test. Prints the staged binary's path.
set -uo pipefail
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 1
binary() {
	local candidate
	for candidate in "${WARP_RUST-}" /workspace/cow/warp-rust-latest "${HOME-}/dev/bin/warp-rust" "${HOME-}/dev/bin/warp"; do
		[ -n "$candidate" ] && [ -x "$candidate" ] || continue
		head -c 2 "$candidate" | grep -q '#!' && continue
		echo "$candidate"
		return
	done
	return 1
}
shared=$(binary) || { echo "shared-warp.sh: no shared warp binary (WARP_RUST names one)" >&2; exit 1; }
stage=$tree/scratch/shared-warp
mkdir -p "$stage" || exit 1
copied() { cmp -s "$1" "$2" || { cp -c "$1" "$2.new-$$" 2>/dev/null || cp --reflink=auto "$1" "$2.new-$$"; } && mv -f "$2.new-$$" "$2" 2>/dev/null; [ -f "$2" ]; }
copied "$shared" "$stage/warp" && chmod +x "$stage/warp" || exit 1
WARP_BINARY=$shared "$tree/scripts/rebuild.sh" --check both "$tree" >&2
if [ "$(uname)" = Darwin ]; then store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}; else store=${WARP_COMPILER_STORE:-/workspace/cow/compilers}; fi
for name in compiler compiler-self; do
	if [ -f "$tree/data/$name.wasm" ]; then copied "$tree/data/$name.wasm" "$stage/$name.wasm"
	elif [ -f "$store/main/$name.wasm" ]; then
		echo "shared-warp.sh: no $name.wasm in data/: the store's main pair, of other compiler inputs" >&2
		copied "$store/main/$name.wasm" "$stage/$name.wasm"
	fi
done
echo "$stage/warp"
