#!/usr/bin/env bash
# GC-free glue (card host-glue-native): under `use native`, print writes an int's digits and a literal text's bytes
# straight from linear memory, and `"…" as cstr` lays a text's bytes and a NUL there, its address an int. Each case
# compares the program's whole output. Run it through the queue: tests/queue.sh tests/host/test_host_native.sh
# The host: WARP_HOST_BINARY (default target/host/warp-host, else target/release/, else target/debug/); the compiler
# warp-host finds for this checkout's lib/ (its data/compiler-self.wasm after `warp rebuild self`, else the one beside
# the host).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
stage=$checkout/scratch/host-native-test
export WARP_LIB=$checkout/lib WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED
rm -rf "$stage" && mkdir -p "$stage" && cd "$stage" || exit 1

run=0 failed=0
# prints <name> <program> <expected output>: the program's stdout and stderr together, exactly
prints() {
	local name=$1 wanted=$3 said
	printf '%s\n' "$2" >program.warp
	said=$("$host" run program.warp 2>&1)
	run=$((run + 1))
	if [ "$said" == "$wanted" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: expected [$wanted], got:"
		echo "$said" | sed 's/^/    /'
	fi
}

prints "an int prints its digits" $'use native\nprint 42' "42"
prints "zero prints one digit" $'use native\nprint 0' "0"
prints "a negative int prints its sign" $'use native\nprint -1234567' "-1234567"
prints "an int computed at run time prints" $'use native\ns = 0\nfor i in 0..<5 { s = s + i }\nprint s' "10"
prints "a literal text prints its bytes" $'use native\nprint "hi"' "hi"
prints "a text and an int print one after the other" $'use native\nprint "answer"\nprint 6 * 7' $'answer\n42'
prints "a cstr holds the bytes and a NUL" $'x = "hi" as cstr\nmemory_byte(x) + memory_byte(x + 1) + memory_byte(x + 2)' "209"
prints "two cstrs keep their own bytes" $'a = "ab" as cstr\nb = "cd" as cstr\nmemory_byte(a) * 1000 + memory_byte(b + 1)' "97100"

cd "$checkout" && rm -rf "$stage"
if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi
