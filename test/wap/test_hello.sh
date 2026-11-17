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
# a form refused by a later stage still names the line the program wrote it on
check "a refusal names the user's line" "$(ran later 'print "a"
print 3' | grep -o 'line [0-9]*:[0-9]*')" "line 2:1"
# a program's value is shown, as warp shows it (notes/self_hosting.md "Value layout": an int is an i64)
check "a program's int value is shown" "$(ran int '40000000000')" "40000000000"
check "a negative int" "$(ran negative '-3')" "-3"
check "the value is the last statement's" "$(ran last 'print "a"
7')" "a
7"
# every kind of literal has its wasm value: a char, a text, ø, a float, then an int shown
check "literals of every kind run" "$(ran kinds "'a'
\"text\"
ø
-2.5
7")" "7"
check "a name the emitter cannot place is refused at its line" "$(ran name 'print "a"
x' | grep -o 'line [0-9]*:[0-9]*')" "line 2:1"
exit $((failures > 0))
