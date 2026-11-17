#!/usr/bin/env bash
# wap, the new compiler, compiles a program to a wasm module that runs (notes/wap.md, notes/backbone.md milestone 1).
# Run it with bash test/wap/test_hello.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/../native/checks.sh"
compiler="$(cd "$(dirname "$0")/../.." && pwd)/wap/compiler.wap"
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT

ran() { # ran <name> <program>: the program compiled by wap, then run, what it printed (or the compiler's error)
	printf '%s\n' "$2" >"$stage/$1.wap"
	$warp "$compiler" "$stage/$1.wap" "$stage/$1.wasm" 2>&1 && $warp "$stage/$1.wasm" 2>&1
}

check "print a text" "$(ran hello 'print "Hello"')" "Hello"
check "statements run in order" "$(ran lines 'print "a"
print "b"')" "a
b"
exit $((failures > 0))
