#!/usr/bin/env bash
# What `warp <code>` shows of its result: a number of an int type as its type's `format … as text` gives it, as print
# writes it (lib/color's red: red, not 4294901760). Run it with bash test/native/test_shown_results.sh; the warp is WARP
# (default the warp on PATH), its lib/ WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
failures=0
check() { # check <code> <expected shown result>
	got=$("$warp" "$1" 2>&1 | tail -1)
	if [ "$got" = "$2" ]; then echo "✓ $1"; else echo "✗ $1: got $got, expected $2"; failures=$((failures + 1)); fi
}
check 'use color; red' red
check 'use color; color(0xFF8000)' '#FF8000'
check 'use color; with_alpha(red, 0.5)' '#FF000080'
check 'type Kolor = int; format Kolor c as text := "K"; 5 as Kolor' K
check 'use color; hex(red)' 4294901760
check 'use color; "red"' '"red"'
check 'class P{x:int; format as text := "P!"}; P(1)' 'P!'
[ $failures = 0 ]
