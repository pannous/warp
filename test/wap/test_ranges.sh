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
exit $((failures > 0))
