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
check "a text's character at a place" "$(ran place 'print "héllo"#2')" "é"
check "each place of a part replaced" "$(ran replace 'print "a-b--c".replace("-", "+") + " " + "x".replace("y", "z")')" "a+b++c x"
check "a text trimmed of blanks at both ends" "$(ran trim 'print "[" + "  a b\t\n".trim() + "]" + "[" + "   ".trim() + "]"')" "[a b][]"
check "a text in lower case" "$(ran lower 'print "Hé ABC".lower()')" "hé abc"
check "a text split at each separator" "$(ran split 'print show("a/bb//c".split("/")) + " " + show("abc".split(", "))')" "[\"a\", \"bb\", \"\", \"c\"] [\"abc\"]"
check "is_letter is another name of is_alpha" "$(ran is_letter 'print text(is_letter("q") and not is_letter("1"))')" "yes"
check "a text's characters from one place to another, counted from 0" "$(ran text_slice 'print "warp"[1 .. 3] + " " + "warp"[1 ..] + " " + "aé€x"[1 to 2]')" "ar arp é€"
check "a text's characters as codes, decoded from UTF-8" "$(ran codepoints 's = "aé€x"
print text(#s.codepoints) + " " + text(ord(s.codepoints#3)) + " " + text(s.codepoints#2)')" "4 8364 é"
check "a text's UTF-8 bytes" "$(ran text_bytes 'print text("aé".bytes) + " " + text(#"".bytes)')" "[97, 195, 169] 0"
check "a text repeated" "$(ran text_times 'print "[" + "ab" * 3 + "][" + "xy" * 0 + "]"')" "[ababab][]"
check "a text's count, also written length or size, counts its characters" "$(ran text_count 'print text("abé".length) + " " + text("".count) + " " + text((1 to 4).size)')" "3 0 4"
check "a character's text, its 2, 3 or 4 UTF-8 bytes" "$(ran char_bytes "print text('ü') + text('€') + text('😀') + ' ' + text(text('😀').bytes)")" "ü€😀 [240, 159, 152, 128]"
exit $((failures > 0))
