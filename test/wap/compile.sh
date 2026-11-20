# the helper of wap's shell tests (source it): ran <name> <program> is what the program printed, compiled by wap, the
# new compiler, and run, or the compiler's error. The warp under test is WARP (default the warp on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/../native/checks.sh"
tree="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_compiler="$tree/wap/compiler.wap"
# the compiler under test: wap's source run by the old compiler, or the module a test script is given as its first
# arguments, --compiler <module> (test/wap/run.sh --stage passes one wap built of itself)
compiler="$source_compiler"
[ "$1" = --compiler ] && { compiler="$2"; shift 2; }
# wap's compiler built by itself <n> times into <folder>: wap1.wasm by the source run by the old compiler, each next
# one by the one before, a module the folder holds already kept; prints the last module, or the failed step's error.
# A module reads its preload beside itself, so the folder holds a copy (the host reads no link out of a folder)
compiler_built() {
	local folder=$1 last=$2 built=$source_compiler n
	mkdir -p "$folder" && cp -R "$tree/wap/preload" "$tree/wap/kernel" "$folder/" || return 1
	for ((n = 1; n <= last; n++)); do
		[ -f "$folder/wap$n.wasm" ] || $warp "$built" "$source_compiler" "$folder/wap$n.wasm" 2>&1 || return 1
		built="$folder/wap$n.wasm"
	done
	echo "$built"
}
# each test script its own folder in the tree's data/ (never the system's temporary folder)
stage="$tree/data/wap-tests/$$"
mkdir -p "$stage" || exit 1
trap 'rm -rf "$stage"' EXIT

ran() {
	printf '%s\n' "$2" >"$stage/$1.wap"
	$warp "$compiler" "$stage/$1.wap" "$stage/$1.wasm" 2>&1 && $warp "$stage/$1.wasm" 2>&1
}
