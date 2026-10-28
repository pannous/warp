#!/usr/bin/env bash
# A primitive file (notes/primitive.md, decision primitive-annotation) built to MVP wasm, made C by wasm2c, compiled by
# cc and run natively prints its value. Run it with bash test/native/test_native.sh; the warp under test is WARP
# (default the warp on PATH), its compiler from WARP_LIB as usual
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
wabt=$(dirname "$(command -v wasm2c)")/..
wasm2c_runtime=$wabt/share/wabt/wasm2c
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

native() { # native <sample> <expected>: the sample run natively prints the expected line
	cp "$checkout/samples/native/$1.warp" "$stage/program.warp"
	(cd "$stage" && "$warp" build program.warp >/dev/null 2>&1)
	[ -f "$stage/program.wasm" ] || { echo "✗ $1: no program.wasm"; failures=$((failures + 1)); return; }
	wasm2c "$stage/program.wasm" -n program -o "$stage/program.c" &&
		cc -O2 -I"$stage" -I"$wasm2c_runtime" -I"$wabt/include" -o "$stage/program" "$stage/program.c" "$checkout/test/native/print_main.c" \
			"$wasm2c_runtime/wasm-rt-impl.c" "$wasm2c_runtime/wasm-rt-mem-impl.c" -lm 2>"$stage/cc.log" ||
		{ echo "✗ $1: wasm2c or cc failed"; head -5 "$stage/cc.log"; failures=$((failures + 1)); return; }
	got=$("$stage/program")
	if [ "$got" = "$2" ]; then echo "✓ $1 natively: $got"; else echo "✗ $1 natively: $got, expected $2"; failures=$((failures + 1)); fi
	got=$(cd "$stage" && "$warp" program.warp 2>&1)
	if [ "$got" = "$2" ]; then echo "✓ $1 by warp: $got"; else echo "✗ $1 by warp: $got, expected $2"; failures=$((failures + 1)); fi
	rm -f "$stage"/program*
}

native fib 832040
exit $failures
