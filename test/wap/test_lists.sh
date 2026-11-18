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
exit $((failures > 0))
