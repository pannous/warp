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
printf '%s\n' 'pattern Pair = (Pair $left: Node $right: Node)' >"$stage/pairs.wap"
check "a used module's pattern builds its shape" "$(ran use_pattern 'use "pairs.wap"
n = Pair(Text("a"), Number(1))
print text(match n { List(items, bracket, separator) => #items; _ => 0 })')" "3"
check "a pattern builds its shape, a rest hole taking a list" "$(ran pattern_rest 'pattern Run = (Run $first: Node $rest...: Node)
n = Run(Text("a"), [Text("b"), Text("c")])
print text(match n { List(items, bracket, separator) => #items; _ => 0 })')" "4"
check "a used module's pattern takes its shape apart in an arm" "$(ran use_pattern_arm 'use "pairs.wap"
n = Pair(Text("a"), Number(1))
print text(match n { Pair(Text(t), right) => t; _ => "no" })')" "a"
check "a pattern's rest hole takes the rest of its shape in an arm" "$(ran pattern_rest_arm 'pattern Run = (Run $first: Node $rest...: Node)
n = Run(Text("a"), [Text("b"), Text("c")])
print text(match n { Run(first, rest) => #rest; _ => 0 })')" "2"
mkdir -p "$stage/inner"
printf '%s\n' 'use "../speak.wap"' 'shout() := say("HEY")' >"$stage/inner/loud.wap"
check "a module in a folder uses one beside its folder, loaded once" "$(ran use_up 'use "inner/loud.wap"
use "speak.wap"
shout()')" "loaded
HEY"
# a name means local > parameter > program > used module > preload; two used modules defining one name clash
printf '%s\n' 'twice(n: int) as int := n * 2' 'show(n: int) as text := "a " + text(n)' >"$stage/a.wap"
printf '%s\n' 'twice(n: int) as int := n + n' >"$stage/b.wap"
check "a used module's word hides the preload's of the same parameters" "$(ran module_over_preload 'use "a.wap"
print(show(3))')" "a 3"
check "a.twice is the used module a's word, beside the program's own twice" "$(ran module_named 'use "a.wap"
twice(n: int) as int := n * 10
print(text(a.twice(3)))
print(text(twice(3)))')" "6
30"
check "two used modules defining one name are refused naming both, even where nothing calls it" "$(ran module_clash 'use "a.wap"
use "b.wap"
print("x")' | grep -o 'a.wap line [0-9]* and b.wap line [0-9]* both define twice')" "a.wap line 1 and b.wap line 1 both define twice"
printf '%s\n' 'twice(n: int) as int := n * 2' >"$stage/c.wap"
check "two used modules defining one word identically are no clash" "$(ran module_identical 'use "a.wap"
use "c.wap"
print(text(twice(4)))')" "8"
check "a variable named like a used module is the variable: a.twice() is twice(a)" "$(ran module_hidden 'use "a.wap"
a = 7
print(text(a.twice()))')" "14"
# a type's name means the record its code sees: a used module's class beside the program's own of that name
printf '%s\n' 'class Box { n: int }' 'boxed(n: int) as text := text(Box(n).n)' >"$stage/boxes.wap"
check "a used module's code keeps its own class where the program declares another of that name" "$(ran module_class 'use "boxes.wap"
class Box { name: text }
print(boxed(3))
print(Box("x").name)')" "3
x"
# a case name two enums share: the case taking as many values as the call gives, though another hides it by scope
printf '%s\n' 'enum Token { Atom, Key }' >"$stage/tokens.wap"
check "a call of a case name picks the case taking its values: Key(a, op, b) is Node's beside a used module's fieldless Key" "$(ran case_by_count 'use "tokens.wap"
shown(n: Node) as text := match n { Key(left, op, right) => "key"; _ => "other" }
token = Key
print(shown(Key(symbol("a"), Op.Colon, Number(1))))
print(if token == Token.Key then "token" else "other")')" "key
token"
check "a module is one module however often a program names it" "$(ran use_named_twice 'use "speak.wap"
use "speak.wap"
say("once")')" "loaded
once"
# include (wiki/use.md): a file's text in place, as it is, so its words are the program's own, and a use inside it
# is read beside the included file
mkdir -p "$stage/parts"
printf '%s\n' 'use "../speak.wap"' 'twice(x: int) as int := x * 2' >"$stage/parts/twice.wap"
check "an included file's definitions are the program's own; its use is found beside it" "$(ran included 'include "parts/twice.wap"
say(text(twice(21)))')" "loaded
42"
exit $((failures > 0))
