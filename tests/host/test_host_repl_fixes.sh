#!/usr/bin/env bash
# warp-host fixes the pure REPL needed (card warp-host-repl-fixes, notes/warp_host.md): run_wasm of an exact rational
# result (card host-run-wasm-float), the user's LC_CTYPE set at startup (libedit's readline reads UTF-8 prompts),
# versioned sonames found without the -dev links (libedit.so.2, libreadline.so.8), and owned_text, a `char *` result
# copied into a text and then freed. Run it through the queue: tests/queue.sh tests/host/test_host_repl_fixes.sh
# The host and compiler are found as tests/host/test_host.sh finds them; staged in scratch/host-repl-test/.
# A library missing on this machine skips its checks (edit, readline).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/host-repl-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED WARP_LIBRARY_PATH

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$(dirname "$rust")/compiler-self.wasm" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild self builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/sonames" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$(dirname "$rust")/compiler-self.wasm" "$stage/bin/" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
# the file of a library's versioned soname on this machine (Linux), else nothing
soname() {
	local file
	for file in /usr/lib/*/lib"$1".so.* /lib/*/lib"$1".so.* /usr/lib64/lib"$1".so.* /usr/lib/lib"$1".so.*; do
		[ -e "$file" ] && { echo "$file"; return 0; }
	done
	return 1
}

# run_wasm of a rational: 2.5 is exact on the pure lane, an Int node holding a $Ratio (it failed "run wasm: 0")
program rational '2.5'
program run_rational 'use os' 'print run_wasm(compile_wasm("2.5"))'
program run_third 'use os' 'print run_wasm(compile_wasm("1/3"))'
program run_rational_back 'use os' 'x = run_wasm(compile_wasm("2.5"))' 'print x * 2' 'print x + 1/4'
program run_negative 'use os' 'print run_wasm(compile_wasm("-7/4"))'
# the locale's character type, which setlocale(LC_CTYPE, NULL) reads back: LC_CTYPE is 0 on Linux, 2 on macOS
ctype=$([ "$(uname)" = Darwin ] && echo 2 || echo 0)
program ctype "setlocale(category: i32, locale: text): text from 'c'" "setlocale($ctype, ø)"
# libraries by their versioned sonames only: libreadline.so.8 (no libreadline.so without the -dev package), and a
# library known only by a soname in WARP_LIBRARY_PATH (libwarpedit.so.2, as libedit.so.2 is without libedit-dev)
program readline "where_history(): i32 from 'readline'" 'where_history()'
program edit_soname "where_history(): i32 from 'warpedit'" 'where_history()'
# owned_text: strdup's copy read back as a text, then freed (free() of what strdup gave; a wrong pointer aborts)
program owned "strdup(s: text): owned_text from 'c'" 'xs = ["owned", "twice"]' 'print strdup(xs#1)' 'print strdup(xs#2)'
program owned_parameter "puts(s: owned_text): i32 from 'c'" 'puts("x")'

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
skip() { skipped=$((skipped + 1)); echo "- $1 ($2)"; }
runs() { "$host" run "$@"; }

expect "a program's exact 2.5 prints as 2.5" 0 "2.5" ! "kind" -- runs rational.warp
expect "run_wasm of a rational result: 2.5" 0 "2.5" ! "run wasm" ! "kind" -- runs run_rational.warp
expect "run_wasm of 1/3 stays exact" 0 "1/3" ! "0.333" -- runs run_third.warp
expect "run_wasm's rational crosses back exact: 5, 2.75" 0 "5" "2.75" ! "Error" -- runs run_rational_back.warp
expect "run_wasm of -7/4: -1.75" 0 "-1.75" -- runs run_negative.warp
expect "LC_CTYPE from the environment (LC_ALL=C.UTF-8)" 0 "UTF-8" -- env LC_ALL=C.UTF-8 "$host" run ctype.warp
expect "LC_CTYPE stays C in the C locale" 0 '"C"' -- env LC_ALL=C "$host" run ctype.warp
if [ "$(uname)" = Linux ] && soname readline >/dev/null; then
	expect "libreadline found by its soname (libreadline.so.8)" 0 "0" ! "not found" -- runs readline.warp
else
	skip readline "no libreadline.so.* here"
fi
if [ "$(uname)" = Linux ] && edit=$(soname edit); then
	ln -s "$edit" "$stage/sonames/libwarpedit.so.${edit##*.so.}"
	expect "a library known only by its versioned soname (libwarpedit.so.2 → $(basename "$edit"))" 0 "0" ! "not found" \
		-- env WARP_LIBRARY_PATH="$stage/sonames" "$host" run edit_soname.warp
else
	skip edit_soname "no libedit.so.* here"
fi
expect "owned_text: strdup's copy is a text, freed after" 0 "owned" "twice" ! "free()" ! "Error" -- runs owned.warp
expect "owned_text is a result type only" 1 "" ! "x" -- runs owned_parameter.warp

echo "host repl fixes: $((run - failed)) passed / $run ($skipped skipped)"
[ "$failed" = 0 ]
