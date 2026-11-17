#!/usr/bin/env bash
# wap's reader (wap/core/parse/reader.wap, notes/reader.md): the cases of test/wap/reader.wap, and a text it cannot
# read refused where it was written. Run it with bash test/wap/test_reader.sh; the warp under test is WARP
source "$(dirname "$0")/../native/checks.sh"
here="$(cd "$(dirname "$0")" && pwd)"
"$warp" --mute "$here/reader.wap" 2>&1 | grep -v '^✓'
cases=${PIPESTATUS[0]}
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
printf 'print "a"\nf(\n' >"$stage/open.wap"
check "an unclosed bracket is refused at its line" "$("$warp" "$here/../../wap/compiler.wap" "$stage/open.wap" "$stage/open.wasm" 2>&1 | grep -o 'line [0-9]*:[0-9]*: this bracket is never closed')" "line 2:2: this bracket is never closed"
exit $((failures > 0 || cases != 0))
