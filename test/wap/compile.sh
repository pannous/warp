# the helper of wap's shell tests (source it): ran <name> <program> is what the program printed, compiled by wap, the
# new compiler, and run, or the compiler's error. The warp under test is WARP (default the warp on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/../native/checks.sh"
tree="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_compiler="$tree/wap/compiler.wap"
# wap's seed, wap built by itself, beside the warp binary with its preload and kernel (scripts/refresh-seed.sh --wap)
seed="$(dirname "$(command -v "$warp")")/wap-seed/wap.wasm"
[ -f "$seed" ] || { echo "no wap seed at $seed: scripts/refresh-seed.sh --wap installs it" >&2; exit 2; }
# wap's compiler built by itself <n> times into <folder>, a module the folder holds already kept; prints the last
# module, or the failed step's error. A module reads its preload beside itself (the host reads no link out of a folder):
# wap0.wasm is the source compiled by the seed beside the seed's preload, so a rule form the tree's preload is the first
# to write is known; wap1.wasm the source compiled by wap0 beside a copy of the tree's preload, each next one by the one
# before. A module is written whole or not at all, so scripts sharing a folder never read half of one
module_made() { [ -f "$2" ] || { $warp "$1" "$source_compiler" "$2.$$" 2>&1 && mv "$2.$$" "$2"; }; }
compiler_built() {
	local folder=$1 last=$2 built=$1/wap0.wasm n
	mkdir -p "$folder" && cp -R "$tree/wap/preload" "$tree/wap/kernel" "$folder/" || return 1
	module_made "$seed" "$built" || return 1
	for ((n = 1; n <= last; n++)); do
		module_made "$built" "$folder/wap$n.wasm" || return 1
		built="$folder/wap$n.wasm"
	done
	echo "$built"
}
# the compiler under test: the module a test script is given as its first arguments, --compiler <module>
# (test/wap/run.sh --stage passes one wap built of itself), else wap1, this tree's source compiled by the seed, built
# once for each state of wap/ and shared by every script
if [ "$1" = --compiler ]; then compiler="$2"; shift 2
else
	sources=$(cd "$tree" && { cat "$seed"; find wap -name '*.wap' | sort | xargs cat; } | shasum | cut -c1-16)
	compiler=$(compiler_built "$tree/data/wap-tests/wap1-$sources" 1) || { echo "wap1: $compiler" >&2; exit 1; }
fi
# each test script its own folder in the tree's data/ (never the system's temporary folder)
stage="$tree/data/wap-tests/$$"
mkdir -p "$stage" || exit 1
trap 'rm -rf "$stage"' EXIT

ran() {
	printf '%s\n' "$2" >"$stage/$1.wap"
	$warp "$compiler" "$stage/$1.wap" "$stage/$1.wasm" 2>&1 && $warp "$stage/$1.wasm" 2>&1
}
