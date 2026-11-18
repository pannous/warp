#!/usr/bin/env bash
# wap's texts compared and joined (wap/preload/prelude.wap; wiki/text.md): `==` and `!=` compare the characters, `+`
# joins two texts. Run it with bash test/wap/test_texts.sh
source "$(dirname "$0")/compile.sh"

check "texts are equal by their characters" "$(ran text_equal 'a = "warp"
if a == "wa" + "rp" then print "same" else print "other"')" "same"
check "texts of other characters or lengths differ" "$(ran text_differ 'if "warp" != "wasp" then print "differ" else print "same"
if "warp" == "war" then print "same" else print "shorter"')" "differ
shorter"
check "two texts joined" "$(ran joined 'greeting(name: text) := "Hello, " + name + "!"
print greeting("wap")')" "Hello, wap!"
# a one-character text is a character (wiki/char.md), and a character is a text wherever one is wanted
check "a character given where a text is wanted" "$(ran char_as_text 'greet() := "x"
print greet()
print "a" + greet()')" "x
ax"
check "a text's bytes and characters' codes by primitives" "$(ran bytes 'code_of(c: char): int := prim i64_extend_i32_u(c)
char_of(n: int): char := prim i32_wrap_i64(n)
byte_count(t: text): int := prim array_len(t)
byte_at(t: text, at: int): int := prim array_get_u(t, at)
bytes_between(t: text, start: int, stop: int): text := prim array_slice(t, start, stop - start)
print text(code_of(char_of(97)))
print text(byte_count("é"))
print text(byte_at("ab", 1))
print bytes_between("hello", 1, 4)')" "97
2
98
ell"
exit $((failures > 0))
