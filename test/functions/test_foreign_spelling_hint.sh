#!/usr/bin/env bash
# another language's spelling of a word, `len(xs)`, keeps working and hints warp's word (lib/aliases.warp's foreign
# field, card compile-time: only foreign spellings hint, warp-interviewer 2026-11-13); a word's English aliases (size)
# and a program's own definition of the spelling stay quiet. Run it with bash test/functions/test_foreign_spelling_hint.sh;
# the warp is WARP (default the warp on PATH), a tree's WARP=<tree>/data/gate/warp
source "$(dirname "$0")/../native/checks.sh"

check "len hints count" "$(hint_of 'xs = [1, 2]; len(xs)')" 'hint: warp says count at 1:14; fix: count'
check "size, an English alias, hints nothing" "$(hint_of 'xs = [1, 2]; size(xs)')" ''
check "a program's own len hints nothing" "$(hint_of 'len(x) := 5; len(3)')" ''
[ $failures = 0 ]
