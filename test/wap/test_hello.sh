#!/usr/bin/env bash
# wap, the new compiler, compiles a program to a wasm module that runs (notes/wap.md, notes/backbone.md milestone 1).
# Run it with bash test/wap/test_hello.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "print a text" "$(ran hello 'print "Hello"')" "Hello"
check "statements run in order" "$(ran lines 'print "a"
print "b"')" "a
b"
# a form refused by a later stage still names the line the program wrote it on
check "a refusal names the user's line" "$(ran later 'print "a"
print 3' | grep -o 'line [0-9]*:[0-9]*')" "line 2:1"
exit $((failures > 0))
