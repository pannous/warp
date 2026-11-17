#!/usr/bin/env bash
# `//` meant as floor division (decision floor-division): glued after an operand and before one or `=`, it starts a
# comment all the same, and the warning names `/!`; a spaced `//` is a plain comment. Run it with
# bash test/native/test_floor_division_comments.sh; the warp is WARP (default the warp on PATH); a tree's:
# WARP=<tree>/data/gate/warp
source "$(dirname "$0")/checks.sh"

check '7//2 warns' "$(warning_of 'x = 7//2; x')" 'warning: x = 7//2; x: // starts a comment, the line ends before it at 1:6; fix: x = 7 /! 2; x'
check 'x//=3 warns' "$(warning_of 'x = 7
x//=3
x')" 'warning: x//=3: // starts a comment, the line ends before it at 2:2; fix: x /!= 3'
check 'a spaced comment does not warn' "$(warning_of 'x = 7 // note
x')" ''
check 'a comment after a call does not warn' "$(warning_of 'print(7) // shown')" ''
check 'a url in a text does not warn' "$(warning_of 'u = "http://x.org"; u')" ''
[ $failures = 0 ]
