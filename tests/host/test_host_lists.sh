#!/usr/bin/env bash
# Long lists from warp-host's words (card warp-host-list-cap): read_bytes of a file over 24,000 bytes, and compile_wasm
# of a `use os` program (a module of ~25 KB, given as a list of its bytes). Before the fix, a list of more than ~20,000
# items failed: "memory cap: growing a memory to 4096 MB …", then "list join: 0". Run it through the queue:
# tests/queue.sh tests/host/test_host_lists.sh
# The host and the compilers as in test_host.sh: WARP_HOST_BINARY (default target/host/warp-host, else target/release/,
# else target/debug/), compiler-self.wasm from beside WARP_LAUNCHER_RUST (default the shared warp, scripts/shared-warp.sh).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$("$checkout"/scripts/shared-warp.sh "$checkout")}
stage=$checkout/scratch/host-lists-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_LIB WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$(dirname "$rust")/compiler-self.wasm" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
cp "$host" "$stage/bin/warp-host" && cp "$(dirname "$rust")/compiler-self.wasm" "$stage/bin/" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
# 30,000 bytes counting 0…255 over and over: their count and their sum say they all arrived
python3 -c 'import sys; sys.stdout.buffer.write(bytes(i % 256 for i in range(30000)))' >bytes.bin || exit 1
printf '%s\n' 'use os' 'use file' 'b = read_bytes("bytes.bin")' 's = 0' 'for x in b { s = s + x }' \
	'print "count " + count(b) + " sum " + s' >read_long.warp
printf '%s\n' 'use os' 'm = compile_wasm("use os\nprint args")' 'print "module " + (count(m) > 24000)' 'run_wasm(m)' \
	>compile_long.warp

run=0 failed=0
# expect <name> <exit status> <text the output holds>… -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() said got text problems=""
	shift 2
	while [ "$1" != -- ]; do wanted+=("$1"); shift; done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}

expect "read_bytes of 30,000 bytes gives them all" 0 "count 30000 sum 3820008" -- "$host" run read_long.warp
expect "compile_wasm of a use os program gives its module, which runs" 0 "module yes" "['a' \"b c\"]" \
	-- "$host" run compile_long.warp a "b c"

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi
