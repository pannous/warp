#!/usr/bin/env bash
# wap builds data as the reader reads it (notes/kernel.md "Data"; wiki/list.md): a list `[a, b]` is a value, `xs#i`
# its i-th item counting from 1, `#xs` how many it holds.
# Run it with bash test/wap/test_data.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a list's item by its place" "$(ran item 'xs = [10, 20, 30]
xs#2')" "20"
check "a list's size" "$(ran size 'xs = [10, 20, 30]
#xs')" "3"
check "a list of texts" "$(ran texts 'names = ["ann", "bo"]
print names#2')" "bo"
check "an item set anew" "$(ran set 'xs = [1, 2]
xs#1 = 5
xs#1 + xs#2')" "7"
check "a list given to a function" "$(ran given 'second(xs: list<int>): int := xs#2
second [4, 5, 6]')" "5"
check "a list's type in words" "$(ran in_words 'second(xs: list of int) as int := xs#2
second [1, 2]')" "2"
check "an item by its index, from 0" "$(ran index 'xs = [10, 20, 30]
xs[0] + xs[2]')" "40"
check "an item set by its index" "$(ran index_set 'xs = [1, 2]
xs[1] = 5
xs#2')" "5"
check "a list of any items counted" "$(ran count_texts 'names = ["ann", "bo", "cy"]
#names')" "3"
check "for over a list of texts" "$(ran for_texts 'for name in ["ann", "bo"] { print name }')" "ann
bo"
check "a compound assignment to a list position" "$(ran compound_item 'xs = [1, 2, 3]
xs#2 *= 5
xs#2')" "10"
check "two lists joined" "$(ran joined 'xs = [1, 2] + [3]
print text(#xs)
xs#3')" "3
3"
check "a map written as data, read by its key" "$(ran map_read 'NAMES = {yes: 1, no: 0}
print text(NAMES.yes)')" "1"
check "a map's value set by its key, a new key added" "$(ran map_set 'm = {a: 1}
m.a = 5
m.b = 7
print text(m.a + m.b)')" "12"
check "an item counted from 0 is a place" "$(ran item_place 'xs = [1, 2]
xs[0] = 5
print text(xs[0] + xs[1])')" "7"
check "a text's character counted from 0" "$(ran text_item 't = "abc"
print t[2]')" "c"
check "a map with text keys, one letter long too" "$(ran text_keys 'm = {"a": 1, "bb": 2}
print text(m["bb"] + m["a"])')" "3"
check "a list grows in place, every name holding it sees the items" "$(ran grown 'xs = [1]
ys = xs
for i in 1 to 9 { xs.add(i) }
print text(#ys)')" "10"
check "the last item taken off a list" "$(ran popped 'xs = [7, 8]
print text(xs.pop())
print text(#xs)')" "8
1"
check "a slice of a list, a new list of its items" "$(ran list_slice 'part(xs: list<int>, from: int, count: int): list<int> := xs.slice(from to from + count - 1)
print show(part([1, 2, 3, 4], 1, 2))')" "[2, 3]"
check "a map's keys, a new list that leaves the map as it was" "$(ran map_keys 'm = {a: 1, b: 2}
ks = m.keys()
ks.add("zz")
print show(ks)
print text(#m.keys())')" "[\"a\", \"b\", \"zz\"]
2"
check "a map's keys in the order set, and whether it has a key" "$(ran map_has 'm = {ab: 1, b: 2}
m.cd = 3
print text(m.has("cd"))
print text(m.has("zz"))
print text(#m.keys())')" "yes
no
3"
check "a map's entry removed, the others kept in their order" "$(ran map_remove 'm = {aa: 1, bb: 2, cc: 3}
m.remove("bb")
m.remove("zz")
m.dd = 4
print text(m.aa + m.cc + m.dd)')" "8"
check "remove takes a key out of a map" "$(ran map_remove_word 'm = {aa: 1, bb: 2}
m.remove("aa")
print text(#m.keys())
print text(m.has("aa"))')" "1
no"
check "an entry set by a key a call gives" "$(ran key_called 'm = {"a": 1}
f(k: text) := k + "b"
m[f("a")] = 2
print text(m["ab"])')" "2"
check "an empty map takes its entries' type from the first one given" "$(ran empty_map_settled 'm = {}
m["a"] = 1
print(text(m["a"]))')" "1"
check "an empty map of the type its place wants" "$(ran empty_map_wanted 't: map<text, text> = {}
t["k"] = "v"
print(t["k"])
class G { names: map<text, int> = {} }
n = G().names
n["b"] = 2
print(text(n["b"]))
count(m: map<text, int>) as int := 7
print(text(count({})))')" "v
2
7"
check "an instance written without a field takes the field's default" "$(ran instance_default 'class P { a: int; b: int = 2 }
p = P{a: 1}
print(text(p.a + p.b))')" "3"
check "a field whose default is ø may be left out" "$(ran empty_default 'class Q { n: int; o: text? = ø }
q = Q(2)
print(text(q.n))
r = Q{n: 4}
print(text(r.o == ø))')" "2
yes"
check "a map's own word read as its part is that word, any other name its entry" "$(ran map_word 'M = {"ab": [1], "cd": [2]}
print(M.keys#2 + "z")
m = {name: "x", first: "y"}
print(m.name + m.first)')" "cdz
xy"
check "an instance made from a map, each field made into its type" "$(ran from_map 'class person { name: text; age: int }
print text(person({name: "Ann", age: "40"}).age + 1)
form = {name: "Bo", age: "7"}
print(person(form).name)
class lock { keys: int }
print text(lock({keys: "3"}).keys)')" "41
Bo
3"
check "an item's own index takes the item first, as a part's does" "$(ran item_index 'xs = [[1, 2, 3], [4]]
print text(xs#1[1 ..])')" "[2, 3]"
check "a map's values, a new list in the order their keys were set" "$(ran map_values 'm = {a: 1, b: 2}
vs = m.values
vs.add(3)
print show(vs)
print text(#m.values())')" "[1, 2, 3]
2"
exit $((failures > 0))
