#!/usr/bin/env bash
# Primitive files (notes/primitive.md, decision primitive-annotation) built by `warp build native` (wasm2c and cc, the
# WASI calls answered by lib/native/wasi.c) run natively and print what they should; those without files also run by
# warp itself. Run it with bash test/native/test_native.sh; the warp under test is WARP (default the warp on PATH), its
# compiler from WARP_LIB as usual
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
native() { # native <sample> <expected> [arguments…]: the sample built natively and run with the arguments prints the expected lines
	sample=$1 expected=$2
	shift 2
	cp "$checkout/samples/native/$sample.warp" "$stage/"
	(cd "$stage" && "$warp" build native "$sample.warp" >"$stage/build.log" 2>&1)
	[ -x "$stage/$sample" ] || { echo "✗ $sample: no executable"; head -5 "$stage/build.log"; failures=$((failures + 1)); return; }
	check "$sample natively" "$(cd "$stage" && "./$sample" "$@")" "$expected"
}
by_warp() { # by_warp <sample> <expected>: warp runs the sample and prints the expected value
	check "$1 by warp" "$(cd "$stage" && "$warp" "$1.warp" 2>/dev/null)" "$2"
}

native fib 832040
by_warp fib 832040
native structs 392
by_warp structs 392
native hello Hello
by_warp hello Hello
native arguments $'one\ntwo words' one "two words"
native files "Hello from a file"
GREETING="Hi there" native c_calls $'Hi there\n8\n5'
check "c_calls by warp refused" "$(cd "$stage" && "$warp" c_calls.warp 2>&1 | grep -c "C calls run by \`warp build native\`")" 1

# the native runner lib/native/driver.warp (notes/primitive.md "Driver"): a sample's module run through wasmtime's C API,
# given on the command line or appended to the driver
cp "$checkout/lib/native/driver.warp" "$stage/"
(cd "$stage" && "$warp" build native driver.warp >"$stage/build.log" 2>&1)
module() { # module <sample>: the sample's wasm module in the stage
	cp "$checkout/samples/native/$1.warp" "$stage/"
	(cd "$stage" && "$warp" build "$1.warp" >/dev/null 2>&1)
}
driven() { # driven <sample> <expected> [arguments…]: the driver runs the sample's module with the arguments
	sample=$1 expected=$2
	shift 2
	module "$sample"
	check "$sample by the driver" "$(cd "$stage" && ./driver "$sample.wasm" "$@")" "$expected"
}
if [ -x "$stage/driver" ]; then
	driven hello Hello
	driven fib 832040
	driven arguments $'one\ntwo words' one "two words"
	driven files "Hello from a file"
	# appended: <driver><module><its length, u64 little-endian>WRPMwasm
	cp "$stage/driver" "$stage/app" && cat "$stage/arguments.wasm" >>"$stage/app"
	python3 -c 'import os, struct, sys; open(sys.argv[1], "ab").write(struct.pack("<Q", os.path.getsize(sys.argv[2])) + b"WRPMwasm")' "$stage/app" "$stage/arguments.wasm"
	check "arguments appended to the driver" "$(cd "$stage" && ./app eins zwei)" $'eins\nzwei'
else
	echo "✗ driver: no executable"; head -5 "$stage/build.log"; failures=$((failures + 1))
fi
exit $failures
