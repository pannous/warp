# the helpers of the wap tests that check what `wap format` prints (source it): format_check <program> <expected>
# reads the program and prints it back in canonical warp, compiling nothing; the warp is WARP (default on PATH)
source "$(dirname "${BASH_SOURCE[0]}")/compile.sh"

formatted() { # formatted <program>: the program as `wap format` prints it (or the compiler's error)
	printf '%s\n' "$1" >"$stage/program.wap"
	$warp "$compiler" format "$stage/program.wap" 2>&1
}
# the program prints as expected
format_check() { check "$1 → $2" "$(formatted "$1")" "$2"; }
