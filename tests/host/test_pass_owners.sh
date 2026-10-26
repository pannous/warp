#!/usr/bin/env bash
# One small program per form of lib/compiler/passes.warp pass_owners (card pass-order-owners): each compiles through
# the pure lane and runs to the value its owner pass makes of it. Compiles only against the staged warp-self, no
# cargo: seconds once the host cache is warm. Run it through the queue: tests/queue.sh tests/host/test_pass_owners.sh
LIBRARY=pass-owners
PRELUDE=()
source "$(dirname "$0")/ffi_library_test.sh"

program foreign 'strlen(s: text): i64 from "c"' 'print strlen("owner")'
program c_import 'import (abs, atoi) from "c"' 'print abs(-3) + atoi("4")'
program runtime_declaration 'floor(x: number): int from warp' 'print floor(7.9)'
program try_form 'v = try 6 * 7 catch e { 0 }' 'print v'
program multiple 'a, b = 20, 22' 'print a + b'
program for_loop 's = 0' 'for i in 1 to 4 { s += i }' 'print s'
program for_map 'm = {a: 1, b: 2}' 'n = 0' 'for k in m { n += m[k] }' 'print n'
program match_form 'x = 2' 'print(match x { 1: "one"; 2: "two"; _: "many" })'
program arm 'x = 3' 'print(match x { 1, 2 => "small"; _ => "big" })'
program enum_form 'enum Coin { Penny, Dime }' 'c = Coin.Dime' 'print(if c == Coin.Dime then "dime" else "other")'
program comprehension 'print [x * x for x in [1, 2, 3]]'
program lambda 'doubled = x => x * 2' 'print doubled(21)'
program number_word 'print two + three'
program fallback 'name = ø' 'print(name ?: "anonymous")'

check foreign "a foreign line: c_imports" 0 "5" ! "Error"
check c_import "an import from c: c_imports" 0 "7" ! "Error"
check runtime_declaration "a declaration from warp: definitions" 0 "7" ! "Error"
check try_form "try: tries" 0 "42" ! "Error"
check multiple "a, b = …: multiple_assignments" 0 "42" ! "Error"
check for_loop "for over a range: loop_heads and iteration_sugar" 0 "10" ! "Error"
check for_map "for over a map's keys: iteration_sugar" 0 "3" ! "Error"
check match_form "match: node_matches" 0 "two" ! "Error"
check arm "an arm of several patterns: first_sugar" 0 "big" ! "Error"
check enum_form "an enum: enums_declared" 0 "dime" ! "Error"
check comprehension "a comprehension: comprehensions" 0 "[1 4 9]" ! "Error"
check lambda "a lambda: lambdas" 0 "42" ! "Error"
check number_word "number words: number_words_pass" 0 "5" ! "Error"
check fallback "a ?: b: sugar's fallbacks" 0 "anonymous" ! "Error"
finish
