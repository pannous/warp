#!/usr/bin/env bash
# wap's enums and variants (notes/kernel.md "Record"; wiki/enum.md, wiki/variant.md): an enum names its cases, each a
# value of the enum's type; a case may carry fields, and match takes it apart.
# Run it with bash test/wap/test_variants.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a case is its own value" "$(ran case 'enum Coin { Penny, Nickel, Dime }
c = Dime
match c { Dime => print "dime"; _ => print "other" }')" "dime"
check "match chooses by case" "$(ran by_case 'enum Coin { Penny, Nickel, Dime }
value(c: Coin): int := match c { Penny => 1; Nickel => 5; Dime => 10 }
value(Nickel) + value(Dime)')" "15"
check "a case with fields taken apart" "$(ran fields 'enum Shape { Circle(r: int), Dot }
area(s: Shape): int := match s { Circle(r) => r * r; Dot => 0 }
area(Circle(3)) + area(Dot)')" "9"
check "cases of one, two and no fields" "$(ran mixed 'enum Tree { Leaf(value: int), Pair(left: int, right: int), Nothing }
sum(t: Tree): int := match t { Leaf(v) => v; Pair(a, b) => a + b; Nothing => 0 }
sum(Pair(3, 4)) + sum(Leaf(5)) + sum(Nothing)')" "12"
check "a case with a text field" "$(ran text_case 'enum Word { Named(name: text), Unnamed }
match Named("hi") { Named(n) => print n; Unnamed => print "none" }')" "hi"
exit $((failures > 0))
