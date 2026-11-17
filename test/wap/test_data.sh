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
exit $((failures > 0))
