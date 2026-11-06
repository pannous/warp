#!/usr/bin/env bash
# the text lints a program as written earns (core/analysis/warnings.data): a text spelling a number added to a number,
# a text's index_of, a type word bound to a value the program also tests with. Run it with bash test/native/test_text_lints.sh; the warp is WARP (default the warp on PATH), its
# lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
unset WARP_HINTS
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
warning_of() { "$warp" "$1" 2>&1 | grep '^warning' ; }

check '"3" + 3 warns' "$(warning_of 'x = "3" + 3; x')" 'warning: "3" + 3 joins the texts; "3" spells a number at 1:5; fix: int("3") + 3 or "3" + str(3)'
check '"a" + 3 does not warn' "$(warning_of 'x = "a" + 3; x')" ''
check "a text's index_of warns" "$(warning_of 't = "abc"; t.index_of("b")')" 'warning: index_of of a text names no unit: it counts characters from 1, as find does; byte_index_of gives the byte offset from 0 at 1:12; fix: find'
check "a list's index_of does not warn" "$(warning_of 'xs = [1, 2]; xs.index_of(2)')" ''
check "a type word bound and tested with is warns" "$(warning_of 'f(n) := { text = "a"; n is text }; f("b")')" 'warning: text holds a value here, so is text and as text compare with it, not with the type text at 1:11; fix: name it e.g. op'
check "a type word bound and never tested does not warn" "$(warning_of 'text = "abc"; #text')" ''
check "a type word local to a function testing nothing does not warn" "$(warning_of 'g(x) := x is text; f() := { text = "a"; #text }; f()')" ''
[ $failures = 0 ]
