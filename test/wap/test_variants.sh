#!/usr/bin/env bash
# wap's enums and variants (notes/kernel.md "Record"; wiki/enum.md, wiki/variant.md): an enum names its cases, each a
# value of the enum's type; a case may carry fields, and match takes it apart.
# Run it with bash test/wap/test_variants.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a case is its own value" "$(ran case 'enum Coin { Penny, Nickel, Dime }
c = Dime
if c == Dime then print "dime" else print "other"')" "dime"
check "match chooses by case" "$(ran by_case 'enum Coin { Penny, Nickel, Dime }
value(c: Coin): int := match c { Penny => 1; Nickel => 5; Dime => 10 }
value(Nickel) + value(Dime)')" "15"
check "a case with fields taken apart" "$(ran fields 'enum Shape { Circle(r: int), Dot }
area(s: Shape): int := match s { Circle(r) => r * r; Dot => 0 }
area(Circle(3)) + area(Dot)')" "9"
check "a union of classes" "$(ran union 'class Circle { r: int }
class Square { side: int }
type Shape = Circle | Square
size(s: Shape): int := match s { Circle(r) => r; Square(side) => side }
size(Square(4)) + size(Circle(1))')" "5"
exit $((failures > 0))
