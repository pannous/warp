#!/usr/bin/env bash
# wap's list words (wap/preload/lists.wap): plain warp over lists and function values, called as `xs.all(…)` or
# all(xs, …). Source: the lead's census of wap's own source (notes/wap.md "Road to self-build"), wiki/list.md.
# Run it with bash test/wap/test_lists.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "every item fits a test" "$(ran all 'print text([1, 2, 3].all(x => x > 0))')" "yes"
check "an item fits a test" "$(ran any 'print text([1, 2, 3].any(x => x > 2))')" "yes"
check "a list holds an item" "$(ran contains 'print text([1, 2, 3].contains(4))')" "no"
check "texts joined by a separator" "$(ran join 'print ["a", "b", "c"].join(", ")')" "a, b, c"
exit $((failures > 0))
