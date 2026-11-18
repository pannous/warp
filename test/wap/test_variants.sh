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
check "two cases of a plain enum differ by ==" "$(ran differ 'enum Fixity { Prefix, Infix, Postfix }
if Prefix == Infix { print "same" } else { print "differ" }')" "differ"
check "a case equals itself" "$(ran equal 'enum Fixity { Prefix, Infix, Postfix }
f = Infix
if f == Infix and f != Postfix { print "same" } else { print "differ" }')" "same"
check "is tests a case with fields by its tag" "$(ran is_case 'enum Shape { Circle(r: int), Dot }
s = Circle(2)
if s is Circle { print "circle" } else { print "other" }')" "circle"
check "a case is of its enum" "$(ran is_enum 'enum Coin { Penny, Nickel, Dime }
if Dime is Coin { print "yes" } else { print "no" }')" "yes"
check "cases of two enums differ" "$(ran two_enums 'enum Coin { Penny, Dime }
enum Fixity { Prefix, Infix }
if Penny == Prefix { print "same" } else { print "differ" }')" "differ"
check "a case pattern tests its fields by their own patterns" "$(ran nested 'enum Shape { Circle(r: int), Dot }
enum Pair { Two(a: Shape, b: int) }
score(p: Pair): int := match p { Two(Dot, n) => n; Two(Circle(r), 0) => r; Two(Circle(r), n) => r * n }
print text(score(Two(Dot, 3)))
print text(score(Two(Circle(5), 0)))
score(Two(Circle(2), 4))')" "3
5
8"
check "a field's pattern may be a value" "$(ran field_value 'enum Shape { Circle(r: int), Dot }
size(s: Shape): int := match s { Circle(1) => 10; Circle(r) => r; Dot => 0 }
size(Circle(1)) + size(Circle(7))')" "17"
check "a case named by its enum" "$(ran qualified 'enum Side { Left, Right }
enum Grouping { Left, Right, Refused }
match Side.Right { Right => print "right"; _ => print "other" }
g = Grouping.Refused
match g { Left => print "left"; Right => print "right"; Refused => print "refused" }')" "right
refused"
check "cases of one name in two enums differ" "$(ran qualified_differ 'enum Side { Left, Right }
enum Grouping { Left, Right }
if Side.Right == Grouping.Right { print "same" } else { print "differ" }')" "differ"
check "cases with fields are equal by their fields" "$(ran fields_equal 'enum Shape { Circle(r: int), Label(name: text), Dot }
if Circle(1) == Circle(2) { print "same" } else { print "differ" }
if Circle(1) == Circle(1) { print "same" } else { print "differ" }
if Label("a") == Label("a") { print "same" } else { print "differ" }
if Circle(1) != Dot { print "differ" } else { print "same" }')" "differ
same
same
differ"
check "a case shows itself as written" "$(ran shown_case 'enum Shape { Circle(r: int), Pair(a: int, b: int), Dot }
print show(Circle(3))
print show(Pair(1, 2))
print show(Dot)')" "Circle(3)
Pair(1, 2)
Dot"
exit $((failures > 0))
