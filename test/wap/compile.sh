# the helper of wap's shell tests (source it): ran <name> <program> is what the program printed, compiled by wap, the
# new compiler, and run, or the compiler's error. The warp under test is WARP (default the warp on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/../native/checks.sh"
tree="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_compiler="$tree/wap/compiler.wap"
# wap's seed, wap built by itself, beside the warp binary with its preload and kernel (scripts/refresh-seed.sh --wap)
seed="$(dirname "$(command -v "$warp")")/wap-seed/wap.wasm"
[ -f "$seed" ] || { echo "no wap seed at $seed: scripts/refresh-seed.sh --wap installs it" >&2; exit 2; }
# wap's compiler built by itself <n> times into <folder>, a folder holding it already kept; prints the module, or the
# failed step's error. A module reads its preload beside itself (the host reads no link out of a folder): wap0.wasm is
# the source compiled by the seed beside the seed's preload, so a rule form the tree's preload is the first to write is
# known; wap1.wasm the source compiled by wap0 beside a copy of the tree's preload, each next one by the one before.
# The folder is built whole beside it and then put in its place, so scripts sharing it never read half a module or a
# preload being copied
compiler_built() {
	local folder=$1 last=$2
	[ -f "$folder/wap$last.wasm" ] || folder_built "$folder" "$last" || return 1
	echo "$folder/wap$last.wasm"
}
folder_built() {
	local building=$1.$$ built=$1.$$/wap0.wasm n
	mkdir -p "$building" && cp -R "$tree/wap/preload" "$tree/wap/kernel" "$building/" || return 1
	$warp "$seed" "$source_compiler" "$built" 2>&1 || return 1
	for ((n = 1; n <= $2; n++)); do
		$warp "$built" "$source_compiler" "$building/wap$n.wasm" 2>&1 || return 1
		built="$building/wap$n.wasm"
	done
	# another script may have put the same folder in place meanwhile: the first one stays
	[ -f "$1/wap$2.wasm" ] && { rm -rf "$building"; return; }
	rm -rf "$1" && mv "$building" "$1"
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
