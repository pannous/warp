#!/usr/bin/env bash
# wap's shown values (wap/preload numbers, texts, lists): a value in warp notation, as old warp shows a program's
# result: texts in quotes with their quote and backslash escaped, characters in single quotes, lists of shown items.
# Run it with bash test/wap/test_show.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "numbers and bools shown" "$(ran numbers 'print show(3) + " " + show(3.5) + " " + show(yes)')" "3 3.5 yes"
check "a text shown in quotes, escaped" "$(ran text 'print show("a\"b\\c")')" '"a\"b\\c"'
check "a character shown in single quotes" "$(ran char "print show('c')")" "'c'"
check "a list shown item by item" "$(ran list 'print show(["ab", "c d"]) + " " + show([1, 2])')" '["ab", "c d"] [1, 2]'
exit $((failures > 0))
