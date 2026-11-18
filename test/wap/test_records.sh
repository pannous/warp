#!/usr/bin/env bash
# wap's records (notes/kernel.md "Record"; wiki/class.md): `class Point { x: int; y: int }` declares a type, its
# constructor Point(x, y) builds an instance, `p.x` reads a field and `p.x = 5` sets it.
# Run it with bash test/wap/test_records.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "an instance's fields" "$(ran fields 'class Point { x: int; y: int }
p = Point(1, 2)
p.x + p.y')" "3"
check "an instance written as data" "$(ran data 'class Point { x: int; y: int }
p = Point{x: 3, y: 4}
p.y')" "4"
check "a field set anew" "$(ran set 'class Point { x: int; y: int }
p = Point(1, 2)
p.x = 5
p.x')" "5"
check "a text field" "$(ran text 'class Person { name: text }
print Person("ann").name')" "ann"
check "an instance given to a function" "$(ran given 'class Point { x: int; y: int }
sum(p: Point): int := p.x + p.y
sum(Point(2, 5))')" "7"
# a class a used module declares, its constructor and fields there for the program
printf '%s\n' 'class Point { x: int; y: int }' >"$stage/points.wap"
check "a used module's class" "$(ran module_class 'use "points.wap"
p = Point(4, 5)
p.y')" "5"
check "a method reads its instance through self" "$(ran method 'class Box { w: int; h: int; area() := self.w * self.h }
b = Box(2, 3)
b.area()')" "6"
check "a method's parameters follow self" "$(ran method_parameters 'class Box { w: int; h: int; scaled(k: int) := self.w * k }
Box(2, 3).scaled(10)')" "20"
check "a method without parentheses is called too" "$(ran method_read 'class Box { w: int; h: int; area() := self.w * self.h }
b = Box(2, 3)
b.area')" "6"
exit $((failures > 0))
