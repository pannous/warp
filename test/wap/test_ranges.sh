#!/usr/bin/env bash
# wap's ranges (wiki/range.md, wiki/keyword.md for): `a to b` includes b, `a .. b` stops before it; both are the
# preload's Range. Run it with bash test/wap/test_ranges.sh
source "$(dirname "$0")/compile.sh"

check "a range includes its end" "$(ran to 'r = 2 to 5
r.last')" "5"
check "upto stops before its end" "$(ran upto 'r = 1 .. 4
r.last')" "3"
check "a range's ends in its other spelling" "$(ran dots 'r = 3…7
r.last - r.from')" "4"
check "upto in words stops before its end" "$(ran upto_word 'r = 1 upto 4
r.last')" "3"
check "a range as an item key slices, open on either side" "$(ran slices 'xs = [1, 2, 3, 4]
print show(xs[2 ..])
print show(xs[.. 2])
print show(xs[1 .. 3])')" "[3, 4]
[1, 2]
[2, 3]"
check "a range mapped, each int's result in order" "$(ran range_map 'print text((0 upto 3).map(at => at * 2)) + " " + text((0 upto 3).map(at => at * 2)#2)')" "[0, 2, 4] 2"
check "a range is a list of its numbers wherever a list is wanted" "$(ran range_as_list 'xs: list<int> = 1 to 3
base = 10
print text(xs) + " " + text((1 to 3).flat_map(at => [at + base])) + " " + text((1 to 3).filter(n => n > 1))')" "[1, 2, 3] [11, 12, 13] [2, 3]"
exit $((failures > 0))
