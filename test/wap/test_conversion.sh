#!/usr/bin/env bash
# How a value gets to another type (wiki/conversion.md: fitting, repacking, making `T(x)`, testing; wiki/truthiness.md,
# wiki/falsey.md: `empty` is the one word a condition asks; wiki/reference.md: `copy`). One line per decided rule.
# Run it with bash test/wap/test_conversion.sh
source "$(dirname "$0")/compile.sh"

check "a float made into an int is cut, written either way" "$(ran cut 'print text(int(2.7))
print text(2.7 as int)')" "2
2"
check "a bool made from a text is read from it" "$(ran bool_read 'print text(bool("no"))
print text(bool("0"))
print text(bool("yes"))')" "no
no
yes"
check "a bool made from any other value is its test" "$(ran bool_test 'print text(bool([]))
print text(bool(3))')" "no
yes"
check "a condition tests a text, it does not read it" "$(ran text_tested 'print text(if "0" then 1 else 2)')" "1"
check "a copy is a list of its own" "$(ran copied 'a = [1]
b = copy(a)
b.add(2)
print text(#a)
print text(#b)')" "1
2"
check "an instance whose class says nothing counts as yes" "$(ran plain_instance 'class Box { items: list<int> = [] }
print(if Box() then "yes" else "no")')" "yes"
check "a range where a list is wanted" "$(ran range_as_list 'size(xs: list) := #xs
print text(size(1 to 3))')" "3"
check "a text and a number join" "$(ran text_plus_number 'print("a" + 3)
print("x·" + 12)')" "a3
x·12"
# decision atom-into-node: a plain atom given where a Node is wanted becomes its case
check "an atom where a Node is wanted is its case, in assignments, arguments and arms" "$(ran atom_node 'shown(n: Node) := show(n)
c: Node = '"'a'"'
k = 2
arm: Node = match k { 1 => "one"; 2 => 2; _ => Symbol("many") }
turned: Node = match k { 1 => Symbol("many"); 2 => 2; _ => "one" }
print shown(c) + " " + shown(3) + " " + shown("hi") + " " + show(arm) + " " + show(turned) + " " + show(if k > 1 then 2.5 else Symbol("x"))')" "Char('a') Number(3) Text(\"hi\") Number(2) Number(2) Number(2.5)"
check "a text of one character is that character" "$(ran char_of_text 'c = "é" as codepoint
print(text(c) + " " + text(c == char(233)))')" "é yes"
# the decided conversions and truthiness (wiki/conversion.md, wiki/truthiness.md)
BAG='class Bag { items: list<int> = [] }
empty(b: Bag): bool := #b.items == 0'
check "an instance made from a map, each field made into its type" "$(ran from_map 'class person { name: text; age: int }
print text(person({name: "Ann", age: "40"}).age + 1)')" "41"
check "a word with several results gives the one that is wanted" "$(ran wanted_result 'hi() as int := 42
hi() as text := "fourtytwo"
t = hi() as int
print text(t)')" "42"
check "a class says when it is empty, and every test follows" "$(ran own_empty "$BAG"'
a = Bag()
b = Bag([1])
print(if a then "full" else "empty")
print(if b then "full" else "empty")
print(if a is empty then "empty" else "full")
print(if not b then "empty" else "full")')" "empty
full
empty
full"
exit $((failures > 0))
