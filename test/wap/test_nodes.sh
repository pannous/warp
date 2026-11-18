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
exit $((failures > 0))
