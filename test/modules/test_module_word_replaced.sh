#!/usr/bin/env bash
# a function of the program with the name and argument count of a used module's replaces it, with a warning; another
# count adds to the module's word, and a module brought without a `use` never warns (wiki/use.md "your own function of
# the same name"). Run it with bash test/modules/test_module_word_replaced.sh; a tree's warp: WARP=<tree>/data/gate/warp
source "$(dirname "$0")/../native/checks.sh"

check "a used module's function replaced warns" "$(warning_of 'use sound
play(x) := 1
play(2)')" 'warning: `play` replaces sound'"'"'s play(tone) at 2:1; fix: rename yours to keep both'
check "another argument count does not warn" "$(warning_of 'use math
clamp(x) := x
clamp(3)')" ''
[ $failures = 0 ]
