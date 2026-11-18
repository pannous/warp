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
check "two lists joined" "$(ran joined '+(a: list<int>, b: list<int>): list<int> := prim array_joined(a, b)
xs = [1, 2] + [3]
print text(#xs)
xs#3')" "3
3"
check "a map written as data, read by its key" "$(ran map_read 'NAMES = {yes: 1, no: 0}
print text(NAMES.yes)')" "1"
check "an item counted from 0 is a place" "$(ran item_place 'xs = [1, 2]
xs[0] = 5
print text(xs[0] + xs[1])')" "7"
check "a text's character counted from 0" "$(ran text_item 't = "abc"
print t[2]')" "c"
exit $((failures > 0))
