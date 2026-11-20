#!/usr/bin/env bash
# wap's list words (wap/preload/lists.wap): plain warp over lists and function values, called as `xs.all(…)` or
# all(xs, …). Source: the lead's census of wap's own source (notes/wap.md "Road to self-build"), wiki/list.md.
# Run it with bash test/wap/test_lists.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "every item fits a test" "$(ran all 'print text([1, 2, 3].all(x => x > 0))')" "yes"
check "an item fits a test" "$(ran any 'print text([1, 2, 3].any(x => x > 2))')" "yes"
check "a list holds an item" "$(ran contains 'print text([1, 2, 3].contains(4))')" "no"
check "texts joined by a separator" "$(ran join 'print ["a", "b", "c"].join(", ")')" "a, b, c"
check "an item's position, 0 for none" "$(ran index_of 'print text([10, 20].index_of(20)) + " " + text([10, 20].index_of(5))')" "2 0"
check "the first item that fits a test" "$(ran index_where 'print text([3, 4, 5].index_where(x => x > 3))')" "2"
check "two lists joined" "$(ran joined 'xs = [1, 2] + [3]
print text(#xs) + " " + text(xs#3)')" "3 3"
check "each item's result" "$(ran map 'print text([1, 2].map(x => x * 10)#2)')" "20"
check "the items that fit a test" "$(ran filter 'print show([1, 2, 3, 4].filter(x => x > 2))')" "[3, 4]"
check "each item's list of results, joined" "$(ran flat_map 'print show([1, 2].flat_map(x => [x, x]))')" "[1, 1, 2, 2]"
check "an item in a list" "$(ran in 'print text(2 in [1, 2]) + " " + text(5 in [1, 2])')" "yes no"
check "the items combined from a first value" "$(ran fold 'print text([1, 2, 3].fold(0, (sum, x) => sum + x)) + " " + [1, 2].fold("", (t, x) => t + text(x))')" "6 12"
check "a row in brackets is a list: [1, 2 / 3, 4] holds two rows" "$(ran rows 'xs = [1, 2
3, 4]
print text(#xs)
print text(xs#2#1)')" "2
3"
check "rows of keys in braces are one map" "$(ran key_rows 'm = {a: 1, b: 2
c: 3}
print text(m["c"])')" "3"
check "a slice takes the items at a range's places, counting from 0, up to the list's end" "$(ran sliced 's = slice([1, 2, 3, 4], Range(1, 9))
print text(#s)
print text(s#1)')" "3
2"
# an empty list's item type is settled by the first use that adds to it
check "an empty list takes its item type from what is added" "$(ran empty_list 'xs = []
for i in 1 to 3 { xs = xs + [i * 2] }
print text(xs#3)
squares(n: int) := {
	found = []
	for i in 1 to n { found += [i * i] }
	found
}
print text(#squares(4))
f(ys: list<int>) := #ys
print text(f([]))')" "6
4
0"
check "an empty list nothing adds to counts no items" "$(ran empty_list_count 'print text(#[])
xs = []
print text(#xs)
xs += [4]
print text(#xs)')" "0
0
1"
check "a one-letter text beside texts is a text" "$(ran list_of_texts 'xs = ["ccc", "a", "bb"]
print xs#2 + "!"')" "a!"
check "the first items of a list, and the rest after them" "$(ran take_drop 'print show([1, 2, 3].take(2)) + " " + show([1, 2, 3].drop(2)) + " " + show([1].take(5)) + " " + show([1].drop(5))')" "[1, 2] [3] [1] []"
check "a list's items summed" "$(ran sum 'print text([1, 2, 3].sum()) + " " + text([0.5, 0.25].sum())')" "6 0.75"
check "a list sorted by a key, equal keys in their order" "$(ran sorted_by 'print show(["ccc", "bb", "aa", "dddd"].sorted(t => #t))')" "[\"bb\", \"aa\", \"ccc\", \"dddd\"]"
check "an empty list typed by its later use" "$(ran later_use 'xs = []
xs.add(2)
print text(xs.sum())
ys = []
print text(#ys)')" "2
0"
check "a list sorted by a key named by" "$(ran sorted_by_named 'print show(["ccc", "bb", "dddd"].sorted(by: t => #t))')" "[\"bb\", \"ccc\", \"dddd\"]"
check "a list's items spread into another list" "$(ran spread 'xs = [1, 2]
ys = [0, ...xs, 4, ...xs]
print text(#ys) + " " + text(ys#5)')" "6 1"
check "a list's last item" "$(ran last 'print "wap/core/x.wap".split("/").last')" "x.wap"
check "a list of lists shown and searched, each word typed for its item type" "$(ran nested 'print show([[1, 2], [3]])
print text([["ab"], ["cd", "ef"]].any(row => row.contains("ef")))')" "[[1, 2], [3]]
yes"
check "an empty list beside lists of ints is one too" "$(ran empty_item 'xs = [[1], []]
ys = xs#2 + [5]
print(text(ys#1 + xs#1#1))')" "6"
check "a list's text is its items' texts in brackets" "$(ran list_text 'xs = [1, 2, 3]
print(text(xs))
print(text([[1], [2, 3]]))')" "[1, 2, 3]
[[1], [2, 3]]"
check "a list's count, also written length or size" "$(ran count 'xs = [4, 5, 6]
print text(xs.count) + " " + text(xs.length) + " " + text([1].size)')" "3 3 1"
check "two lists are equal when their items are, in order" "$(ran list_equal 'print text([1, 2] == [1, 2]) + " " + text([1, 2] == [2, 1]) + " " + text(["a"] != ["a", "b"])')" "yes no yes"
check "a list's items joined by their texts" "$(ran join_any 'print [1, 2].join(" ") + "|" + "ab".codepoints.join("-") + "|" + ["x", "y"].join(", ")')" "1 2|a-b|x, y"
check "a copied list grows apart from the first" "$(ran list_copy 'a = [1]
b = copy(a)
b.add(2)
c = a
c.add(3)
print text(#a) + " " + text(#b)')" "2 2"
check "a list's items without their repeats, each where it first appears" "$(ran unique_items 'print(text([3, 1, 3, 2, 1].unique))')" "[3, 1, 2]"
check "an optional value is in a list when it is there, a missing one in none" "$(ran optional_in 'found(n: int) as int? := if n > 0 then n else ø
print(text(found(2) in [1, 2]) + " " + text(found(3) in [1, 2]) + " " + text(found(0) in [1, 2]) + " " + text([1, 2].contains(found(1))))')" "yes no no yes"
check "a missing value is in a list that holds a missing value" "$(ran missing_in 'found(n: int) as int? := if n > 0 then n else ø
print(text(found(0) in [found(1), found(0)]) + " " + text(found(0) in [found(1)]))')" "yes no"
check "a list finds its first item that fits a test, none when no item does" "$(ran list_find 'xs = [1, 2, 3]
print(text(xs.find(x => x > 1) ?? 0) + " " + text(xs.find(x => x > 5) ?? 0) + " " + (["a", "bc"].find(t => #t == 2) ?? "none"))')" "2 0 bc"
check "a list of optional ints takes an int, which it holds as an int?" "$(ran add_optional 'ys = [ø, 2]
ys.add(3)
print(text(#ys) + " " + text(ys#3 ?? 0))')" "3 3"
check "an empty list where an optional list is the result is a list of the item type it holds" "$(ran optional_empty 'picked(n: int) as list<int>? := {
	if n < 0 { return ø }
	if n == 0 { return [] }
	[n]
}
print(text(picked(0)) + " " + text(picked(2)) + " " + text(picked(-1) == ø))')" "[] [2] yes"
check "a function of any list called on a list before its item type settles and on one of that type is one function" "$(ran settled_alike 'a = []
b = ["x"]
print(text(empty(a)) + " " + text(empty(b)))
a += ["y"]
print(text(empty(a)))')" "yes no
no"
check "a list of a shape's values joins a list of the type it narrows" "$(ran shape_join 'xs: list<Node> = [Symbol("b")]
ys = [Text("a")] + xs
print(text(#ys))')" "2"
check "a program's own length and count are two names; size still reads as count" "$(ran own_length 'scaled(length: int, count: int): int := length * 10 + count
length = 3
count = 4
print text(scaled(length, count)) + " " + text(length) + " " + text([1, 2].size)')" "34 3 2"
check "a list of items that have an order sorts by itself" "$(ran sorted_items 'keys: list<text> = ["b", "c", "a"]
print text(keys.sorted()) + " " + text([3, 1, 2].sorted())')" "[a, b, c] [1, 2, 3]"
exit $((failures > 0))
