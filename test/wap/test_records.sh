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
check "a method reads and sets its receiver's fields by their bare names, a parameter hiding one" "$(ran bare_fields 'class Counter { c: int
inc() := { c = c + 1; c }
}
print text(Counter(4).inc())
k = Counter(1)
k.inc()
k.inc()
print text(k.c)
class Box { w: int
h: int
area() := w * h
scaled(w) := w * h
}
b = Box(2, 3)
print text(b.area())
b.scaled(10)')" "5
3
6
30"
check "a class of one field" "$(ran one_field 'class P { x: int }
P(3).x')" "3"
check "an instance shows itself in its constructor form" "$(ran shown 'class P { x: int }
class Q { p: P; name: text }
print show(Q(P(1), "a"))')" 'Q{p: P{x: 1}, name: "a"}'
check "a field's item read and set through the dot" "$(ran field_item 'class G { names: map<text, int> }
g = G({"a": 1})
g.names["b"] = 2
print(text(g.names["a"] + g.names["b"]))
class H { xs: list<int> }
h = H([5, 6])
h.xs[0] = 7
print(text(h.xs[0] + h.xs[1]))')" "3
13"
check "a field of a class type takes ø" "$(ran field_null 'class Item { n: int }
class Holder { item: Item? = ø }
h = Holder(Item(1))
h.item = ø
print text(h.item == ø)')" "yes"
check "records that refer to each other" "$(ran each_other 'class Item { n: int; box: Box? }
class Box { item: Item }
i = Item(7, ø)
b = Box(i)
i.box = b
print text(b.item.n)')" "7"
check "a record holding a list of itself, the list given to a function" "$(ran own_list 'class Tree { n: int; kids: list<Tree> }
count(kids: list<Tree>) as int := #kids
t = Tree(1, [Tree(2, []), Tree(3, [])])
print text(count(t.kids))')" "2"
exit $((failures > 0))
