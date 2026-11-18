#!/usr/bin/env bash
# wap's text words (wap/preload/texts.wap): characters counted and taken, searched, compared, and their kinds, as
# warp's runtime text words give them (core/runtime/text_words.warp; wiki/string.md).
# Run it with bash test/wap/test_texts_words.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a text's characters counted" "$(ran count 'print text(#"héllo")')" "5"
check "where a part stands" "$(ran find 'print text("abcab".find("ca"))')" "3"
check "a text holds a part" "$(ran contains 'print text("abc".contains("x"))')" "no"
check "a text starts and ends with parts" "$(ran ends 'print text("hello".starts_with("he") and "hello".ends_with("lo"))')" "yes"
check "texts in order" "$(ran order 'print text("ab" < "b")')" "yes"
check "the kinds of characters" "$(ran kinds 'print text(is_digit("7") and is_alpha("q") and is_hex_digit("F") and not is_digit("x"))')" "yes"
exit $((failures > 0))
