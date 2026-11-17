#!/usr/bin/env bash
# a number glued to an i the program never binds, `.16i`, keeps its unknown-name error and gains the hint naming ⅈ, the
# imaginary unit (card imaginary-i-hint, OK 2026-11-12); a program's own i stays its own. Run it with
# bash test/numbers/test_imaginary_i_hint.sh; the warp is WARP (default the warp on PATH), a tree's WARP=<tree>/data/gate/warp
source "$(dirname "$0")/../native/checks.sh"

check "a glued i the program never binds gets the hint" "$(hint_of 'use complex; -.8 + .16i')" 'hint: did you mean ⅈ, the imaginary unit? at 1:20; fix: 0.16ⅈ'
check "a program's own i gets no hint" "$(hint_of 'i = 3; 2i')" ''
[ $failures = 0 ]
