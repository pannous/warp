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
exit $failures
