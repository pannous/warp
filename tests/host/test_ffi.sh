#!/usr/bin/env bash
# C calls of pure programs (card pure-c-imports, notes/warp_host.md "C calls"): compiler-self.wasm compiles foreign
# lines, `import f from 'lib'` and `use lib` into imports with a warp.c_functions section, and warp-host loads the
# libraries and calls them. Run it through the queue: tests/queue.sh tests/host/test_ffi.sh
# The host and compilers are found as tests/host/test_host.sh finds them; WARP_FFI_COMPILER=full tests compiler.wasm,
# WARP_FFI_COMPILER_FILE names the compiler module to use.
# A library missing on this machine skips its checks (sqlite3, z).
# The texts are longer than one letter: the pure lane prints a one-letter text in a list as a codepoint ('a'), C's or not.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/ffi-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED WARP_CRATES_CACHE

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
compiler=${WARP_FFI_COMPILER:-self}
compiler_file=$([ "$compiler" = full ] && echo compiler.wasm || echo compiler-self.wasm)
compiler_path=${WARP_FFI_COMPILER_FILE:-$(dirname "$rust")/$compiler_file}
[ -f "$compiler_path" ] || { echo "✗ no $compiler_file beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$compiler_path" "$stage/bin/$compiler_file" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
# whether the C library is on this machine (Linux sonames, macOS dylibs)
has_library() {
	local file
	for file in /usr/lib/*/lib"$1".so* /lib/*/lib"$1".so* /usr/lib/lib"$1".so* /usr/lib64/lib"$1".so* \
		/opt/homebrew/lib/lib"$1".dylib /usr/local/lib/lib"$1".dylib; do
		[ -e "$file" ] && return 0
	done
	# macOS keeps libz and libsqlite3 in the dyld cache, no file
	[ "$(uname)" = Darwin ] && { [ "$1" = z ] || [ "$1" = sqlite3 ]; }
}

program strlen 'import strlen from "c"' 'strlen("hello")'
program foreign_line "erf(x: float): float from 'm'" 'erf(1.0)'
program libm_unimported 'fmin(3.5, 2.1)'
program libm_group "import (ceil, floor, fmax) from 'm'" 'fmax(ceil(2.3), floor(5.9))'
program getenv 'import getenv from "c"' 'getenv("WARP_FFI_TEST")'
program getenv_unset 'import getenv from "c"' 'x = getenv("WARP_FFI_NO_SUCH_VARIABLE")' 'if x == ø then "unset" else x'
program text_at_run_time 'import atoi from "c"' 'xs = ["12", "30"]' 'atoi(xs#1) + atoi(xs#2)'
program use_c 'use c' 'toupper(97)'
program f32 "sqrtf(x: f32): f32 from 'm'" 'sqrtf(16)'
program unknown_import 'import no_such_function from "c"' 'no_such_function(1)'
program unknown_library "no_such_function(x: i32): i32 from 'no_such_library'" 'no_such_function(1)'
program sqlite_raw 'use sqlite3' 'db = sqlite3_open(":memory:")' 'statement = sqlite3_prepare_v2(db, "select 1+2", -1)' \
	'sqlite3_step(statement)' 'sqlite3_column_int(statement, 0)'
program sqlite_rows 'use sqlite3' 'db = sqlite3_open(":memory:")' 'print rows(db, "select 1+2, '"'"'ab'"'"', 2.5, null")'
program sqlite_table 'use sqlite3' 'db = sqlite3_open(":memory:")' 'run_query(db, "create table t(name text, n int)")' \
	'run_query(db, "insert into t values ('"'"'xy'"'"', 1), ('"'"'yz'"'"', 2)")' 'print rows(db, "select name, n * 10 from t order by n")' \
	'sqlite3_close(db)'
program sqlite_errstr 'use sqlite3' 'sqlite3_errstr(1)'
program zlib "compressBound(length: i64): i64 from 'z'" 'compressBound(100)'
program zlib_text "adler32(start: i64, data: text, length: i32): i64 from 'z'" 'adler32(1, "abc", 3)'
# bytes: a pointer and the length a `length` parameter gets, so a NUL doesn't end them
program zlib_bytes "crc32(start: i64, data: bytes, size: length): i64 from 'z'" 'crc32(0, [97, 0, 98])'
program zlib_bytes_text "crc32(start: i64, data: bytes, size: length): i64 from 'z'" 'crc32(0, "abc")'
# a crate's library and native file where scripts/warp-crate installs them, ~/.cache/warp/crates/{lib,native}
# (notes/crate_adapter.md); libm stands in for a crate's library
crates=$stage/home/.cache/warp/crates
mkdir -p "$crates/lib" "$crates/native"
printf '%s\n' "nextafter(x: f64, y: f64): f64 from 'rs_m'" 'towards_zero(x: float) := nextafter(x, 0.0)' >"$crates/native/rs_m.warp"
libm=$(ls /lib/*/libm.so.6 /usr/lib/*/libm.so.6 /usr/lib64/libm.so.6 2>/dev/null | head -1)
[ -n "$libm" ] && ln -s "$libm" "$crates/lib/librs_m.so"
program crate_use 'use crate "m@1"' 'print nextafter(1.0, 2.0)'
program crate_native 'use rs_m' 'print(if towards_zero(1.0) < 1.0 then "below" else "not below")'

run=0 failed=0 skipped=0
# expect <name> <exit status> <text the output holds>… [! <text it lacks>…] -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() unwanted=() said got
	shift 2
	while [ "$1" != -- ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	local text problems=""
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
skip() { skipped=$((skipped + 1)); echo "- $1 (no lib$2 here)"; }
runs() { "$host" --compiler "$compiler" run "$@"; }

expect "libc strlen by import: 5, the exit status" 5 "5" -- runs strlen.warp
expect "a foreign line declares libm's erf" 0 "0.84270079294971" -- runs foreign_line.warp
expect "libm's fmin needs no import" 0 "2.1" ! "Error" -- runs libm_unimported.warp
expect "a char * result is a text" 0 "from the test" -- env WARP_FFI_TEST="from the test" "$host" --compiler "$compiler" run getenv.warp
expect "a NULL char * is ø" 0 "unset" -- runs getenv_unset.warp
expect "a text known only at run time crosses" 42 "42" -- runs text_at_run_time.warp
expect "use c: libc's lines (toupper)" 65 "65" -- runs use_c.warp
expect "an import of no known line is refused, naming it" 1 "no C signature for no_such_function from 'c'" -- runs unknown_import.warp
expect "a missing library fails the run, naming it" 1 "no_such_library" -- runs unknown_library.warp
if has_library sqlite3; then
	expect "sqlite3 by its C functions: select 1+2" 3 "3" -- runs sqlite_raw.warp
	expect "sqlite3 rows: an Int, a text, a float, ø" 0 '3' '"ab"' '2.5' -- runs sqlite_rows.warp
	expect "sqlite3 run_query and rows of a table" 0 '"xy"' '10' '"yz"' '20' ! "Error" -- runs sqlite_table.warp
	expect "sqlite3_errstr: a C text result" 0 "SQL logic error" -- runs sqlite_errstr.warp
else
	for name in sqlite_raw sqlite_rows sqlite_table sqlite_errstr; do skip $name sqlite3; done
fi
if has_library z; then
	expect "zlib by a foreign line, its versioned soname found" 113 "113" -- runs zlib.warp
	expect "zlib adler32 of a text: 38600999, its low byte the status" 39 "38600999" -- runs zlib_text.warp
	expect "bytes and their length: crc32 of a, NUL, b is 367556721" 113 "367556721" -- runs zlib_bytes.warp
	expect "a text as bytes: crc32 of abc is 891568578" 194 "891568578" -- runs zlib_bytes_text.warp
else
	for name in zlib zlib_text zlib_bytes zlib_bytes_text; do skip $name z; done
fi
if [ -n "$libm" ]; then
	expect "use crate \"m@1\": rs_m's native file and library from ~/.cache/warp/crates" 0 "1.0000000000000002" -- \
		env HOME="$stage/home" "$host" --compiler "$compiler" run crate_use.warp
	expect "use rs_m: a crate's warp words over its lines" 0 "below" ! "not below" ! "Error" -- \
		env HOME="$stage/home" "$host" --compiler "$compiler" run crate_native.warp
else
	for name in crate_use crate_native; do skip $name m.so.6; done
fi

echo "ffi: $((run - failed)) passed / $run ($skipped skipped)"
[ "$failed" = 0 ]
