#!/usr/bin/env bash
# wap's Node (wap/preload/nodes.wap): warp's node built and taken apart in plain warp (core/node.warp), its position
# and facts kept in a Meta, as wap's own stages read them.
# Run it with bash test/wap/test_nodes.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a symbol by its name" "$(ran symbol 'print match symbol("x") { Symbol(name) => name; _ => "other" }')" "x"
check "a node's position" "$(ran position 'n = positioned(Text("a"), 3, 7)
print text(line_of(n)) + ":" + text(column_of(n))')" "3:7"
check "a node without a position" "$(ran no_position 'print text(line_of(Text("a")) == ø)')" "yes"
check "a fact added keeps the position" "$(ran facts 'n = with_attribute(positioned(Symbol("x"), 2, 1), Text("f"))
print text(#attributes_of(n)) + " " + text(line_of(n))')" "1 2"
check "a node shows its parts" "$(ran shown_node 'print show(List([Symbol("f"), Text("a")], Bracket.Round, Separator.Space))')" 'List([Symbol("f"), Text("a")], Round, Space)'
check "a node's text, as warp prints it" "$(ran node_text 'a = symbol("a")
print "${a} ${Key(a, Op.Colon, Number(1))} ${Key(symbol("f"), Op.None, List([a], Bracket.Round, Separator.Comma))} ${Key(a, Op.Add, a)} ${Key(a, Op.And, a)}"
print text(List([Text("x"), Number(2.5)], Bracket.Square, Separator.Comma)) + " " + text(List([a, a], Bracket.Curly, Separator.Semicolon))')" "a a: 1 f(a) a+a a and a
[\"x\", 2.5] {a; a}"
check "two nodes are equal wherever they were written" "$(ran node_equal 'x = symbol("x")
print text(positioned(x, 4, 2) == x) + " " + text([positioned(x, 1, 1)] == [x]) + " " + text(Key(positioned(x, 2, 2), Op.Add, x) == Key(x, Op.Add, x)) + " " + text(x == Text("x")) + " " + text(positioned(x, 1, 1) != symbol("y"))')" "yes yes yes no yes"
check "a node is empty as warp reads it in a condition" "$(ran node_empty 'a = symbol("a")
print [Empty, False, Number(0), Text(""), List([], Bracket.Square, Separator.Comma), Key(Text(""), Op.Colon, Number(0)), positioned(Number(0), 1, 1)].map(n => if n then "t" else "f").join("")
print [True, a, Number(2), Text("x"), List([a], Bracket.Round, Separator.Space), Key(a, Op.Colon, Number(0))].map(n => if n then "t" else "f").join("")')" "fffffff
tttttt"
check "a Node pattern looks through the Meta a node's position wraps it in; a Meta pattern sees the Meta" "$(ran through_meta 'n = positioned(Text("hi"), 1, 1)
print(match n { Text(t) => t; _ => "other" })
print(match n { Meta(inner, data) => "meta"; _ => "other" })')" "hi
meta"
exit $((failures > 0))
