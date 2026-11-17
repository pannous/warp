# the helpers of the wap tests that check what `wap format` prints (source it): format_check <program> <expected>
# compiles nothing, it reads the program and prints it back in canonical warp; the warp is WARP (default on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/../native/checks.sh"
compiler="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/wap/compiler.wap"
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT

formatted() { # formatted <program>: the program as `wap format` prints it (or the compiler's error)
	printf '%s\n' "$1" >"$stage/program.wap"
	$warp "$compiler" format "$stage/program.wap" 2>&1
}
# the program prints as expected
format_check() { check "$1 → $2" "$(formatted "$1")" "$2"; }
