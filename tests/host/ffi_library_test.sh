# Sourced by the tests of a C library in pure warp (tests/host/test_ffi_<library>.sh): stages warp-host and
# compiler-self.wasm as tests/host/test_ffi.sh does, in scratch/ffi-<library>-test/, and gives
#   has_library <name>              whether lib<name> is on this machine
#   program <name> <source lines…>  <name>.warp in the stage, after the lines of $PRELUDE
#   check <program> <description> <exit status> <text the output holds>… [! <text it lacks>…]
#   finish                          the count, and the status: 0 when every check passed
# Set LIBRARY (the stage's name) and PRELUDE (lines before each program) first.
checkout=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/ffi-$LIBRARY-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED WARP_CRATES_CACHE

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler_path=${WARP_FFI_COMPILER_FILE:-$(dirname "$rust")/compiler-self.wasm}
[ -f "$compiler_path" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild self builds it"; exit 1; }
has_library() {
	local file
	for file in /usr/lib/*/lib"$1".so* /lib/*/lib"$1".so* /usr/lib64/lib"$1".so* /usr/lib/lib"$1".so* \
		/opt/homebrew/lib/lib"$1".dylib /usr/local/lib/lib"$1".dylib; do
		[ -e "$file" ] && return 0
	done
	# macOS keeps libz and libsqlite3 in the dyld cache, no file
	[ "$(uname)" = Darwin ] && { [ "$1" = z ] || [ "$1" = sqlite3 ]; }
}
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$compiler_path" "$stage/bin/compiler-self.wasm" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() {
	local name=$1
	shift
	printf '%s\n' "${PRELUDE[@]}" "$@" >"$stage/$name.warp"
}
run=0 failed=0
check() {
	local program=$1 name=$2 status=$3 wanted=() unwanted=() said got text problems=""
	shift 3
	while [ $# -gt 0 ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	said=$("$host" --compiler self run "$program.warp" 2>&1)
	got=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	for text in "${unwanted[@]}"; do [[ $said == *"$text"* ]] && problems+=" unwanted [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}
finish() {
	echo "$LIBRARY: $((run - failed)) passed / $run"
	[ "$failed" = 0 ]
}
