# the helper of wap's shell tests (source it): ran <name> <program> is what the program printed, compiled by wap, the
# new compiler, and run, or the compiler's error. The warp under test is WARP (default the warp on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/../native/checks.sh"
tree="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compiler="$tree/wap/compiler.wap"
# each test script its own folder in the tree's data/ (never the system's temporary folder)
stage="$tree/data/wap-tests/$$"
mkdir -p "$stage" || exit 1
trap 'rm -rf "$stage"' EXIT

ran() {
	printf '%s\n' "$2" >"$stage/$1.wap"
	$warp "$compiler" "$stage/$1.wap" "$stage/$1.wasm" 2>&1 && $warp "$stage/$1.wasm" 2>&1
}
