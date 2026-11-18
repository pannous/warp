#!/usr/bin/env bash
# wap's modules: `use "file"` brings in a file's definitions and runs its statements first (notes/backbone.md "Stage
# languages", declared; core/analyze/declarations.wap). Run it with bash test/wap/test_modules.sh
source "$(dirname "$0")/compile.sh"

printf '%s\n' 'say(t: text) := print(t)' 'say("loaded")' >"$stage/speak.wap"
printf '%s\n' 'use "speak.wap"' 'greeting = "hi"' 'welcome() := say(greeting)' >"$stage/greet.wap"

check "a used module's function, its statements run first" "$(ran use_one 'use "speak.wap"
say("Hello")')" "loaded
Hello"
check "a module uses its own modules and variables" "$(ran use_nested 'use "greet.wap"
welcome()')" "loaded
hi"
check "a module used twice runs once" "$(ran use_twice 'use "greet.wap"
use "speak.wap"
welcome()
say("both")')" "loaded
hi
both"
printf '%s\n' 'use "pong.wap"' 'ping() := print("ping")' 'serve() := rally()' >"$stage/ping.wap"
printf '%s\n' 'use "ping.wap"' 'pong() := print("pong")' 'rally() := {' '	ping()' '	pong()' '}' >"$stage/pong.wap"
check "modules may use each other" "$(ran use_each_other 'use "ping.wap"
serve()')" "ping
pong"
# a module may hold only rules
printf '%s\n' 'rule twice $x => $x + $x' >"$stage/doubling.wap"
check "a module of rules only" "$(ran use_rules 'use "doubling.wap"
print "used"')" "used"
# a used module's rules build the language the program is written in (wiki kernel.md)
check "a used module's rule" "$(ran use_rule 'use "doubling.wap"
twice 4')" "8"
# a used module's enums and classes are the program's to use; a program's own case hides the module's
printf '%s\n' 'enum Tree { Leaf(value: int), Named(name: text), Nothing }' 'class Box { n: int }' >"$stage/trees.wap"
check "a used module's case with fields" "$(ran use_cases 'use "trees.wap"
print match Named("x") { Named(name) => name; _ => "o" }')" "x"
check "a used module's class and bare case" "$(ran use_class 'use "trees.wap"
b = Box(2)
t = Tree.Nothing
print text(b.n + 3)')" "5"
check "a program's own case hides the module's" "$(ran use_hidden_case 'use "trees.wap"
enum Side { Leaf, Root }
print text(Side.Leaf == Leaf)')" "yes"
check "a used module's case matched with its field" "$(ran use_case_field 'use "trees.wap"
leaf = Leaf(3)
print match leaf { Leaf(v) => text(v + 1); _ => "o" }')" "4"
exit $((failures > 0))
