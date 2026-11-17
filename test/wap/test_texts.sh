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
exit $((failures > 0))
