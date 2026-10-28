#!/usr/bin/env bash
# `warp build` of a GC program writes an executable, the warp-runtime stub carrying its machine code, which prints the
# program's value and calls C as warp does (texts, handles). Run it with bash test/native/test_executables.sh; the warp
# under test is WARP (default the warp on PATH), its stub beside it or in WARP_RUNTIME_STUB
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

built() { # built <name> <expected> <code>: the program built as an executable prints the expected text
	printf '%b\n' "$3" >"$stage/$1.warp"
	(cd "$stage" && "$warp" build "$1.warp" >"$stage/build.log" 2>&1)
	[ -x "$stage/$1" ] || { echo "✗ $1: no executable"; tail -2 "$stage/build.log"; failures=$((failures + 1)); return; }
	got=$("$stage/$1")
	if [ "$got" = "$2" ]; then echo "✓ $1"; else echo "✗ $1: got $got, expected $2"; failures=$((failures + 1)); fi
}

built hello "Hello 🌍2026" '"Hello " + "🌍" + (2000 + 26)'
built printing "hi" 'print "hi"'
built text_to_c 5 'use c\nstrlen("hello")'
built handles warp "use sqlite3\ndb = sqlite3_open(\":memory:\")\nstmt = sqlite3_prepare_v2(db, \"select 'warp'\", -1)\nsqlite3_step(stmt)\nsqlite3_column_text(stmt, 0)"
exit $((failures > 0))
