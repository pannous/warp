#!/usr/bin/env bash
# wap's binding analysis: what each name refers to is decided once, by its scope (notes/first_principles.md 15;
# wiki/Footguns.md "Parameter shadowing": a local named like a library word hides it only in its own function).
# Run it with bash test/wap/test_binding.sh
source "$(dirname "$0")/compile.sh"

# a function's local never leaks into the rest of the program, whatever its name (the old compiler's push-shadowing bug)
check "a local named like a library word stays in its function" "$(ran local 'quiet() := { print = "x" }
print "Hello"')" "Hello"
# a typed parameter binds its name, its type left as written
check "a typed parameter" "$(ran typed 'keep(x: int) := { y = x }
print "Hello"')" "Hello"
# wiki/Footguns.md "Closures capturing loop variables": a function's `x = …` of a program's variable reads two ways;
# `let` makes the function its own, `global` changes the program's
check "let makes a new variable" "$(ran let 'x = 1
f() := { let x = 2 }
f()
x')" "1"
check "global changes the program's" "$(ran global 'x = 1
f() := { global x = 2 }
f()
x')" "2"
exit $((failures > 0))
