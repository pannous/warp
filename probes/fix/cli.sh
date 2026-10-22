#!/bin/zsh
# warp fix and warp beautify run lib/fix/fix.warp (card fix-in-lib): a file fixed in place, previewed without `yes`,
# inline code printed. usage: probes/fix/cli.sh [warp binary]   (prints ok or the failing step, exit 1)
warp=${1:-warp}
folder=$(mktemp -d)
file=$folder/a.warp
fail() { print -r -- "✗ $1"; exit 1 }
print -n -- '// a note\nx=1+2\nys = float[2]\nx\n' > $file
$warp --no-ask beautify $file | grep -q -- '+# a note' || fail "beautify previews the diff"
[[ $(<$file) == $'// a note\nx=1+2\nys = float[2]\nx' ]] || fail "a preview applies nothing"
$warp --no-ask fix yes $file > /dev/null || fail "warp fix yes exits 0"
[[ $(<$file) == $'# a note\nx = 1 + 2\nys = 2 floats\nx' ]] || fail "warp fix yes writes the fixed file: $(<$file)"
$warp --no-ask fix check $file > /dev/null || fail "a fixed file has nothing to fix"
$warp --no-ask beautify 'use script; ls // the folder' | grep -q 'ls # the folder' || fail "inline code is printed fixed"
print ok
