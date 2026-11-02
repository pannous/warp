#!/usr/bin/env bash
# A file run links the prebuilt modules it uses; their classes' shared method names (sound's Note, reals' Real and
# units' Quantity each declare text and show) bind, with no warning: a link that is still refused says so in one line
# ("the linked module didn't bind"), never silently (card bind-linked). Run it with bash test/modules/test_linked_binds.sh;
# the warp under test is WARP (default the warp on PATH)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
check() { # check <name> <program> <expected output>
	printf '%s\n' "$2" >"$stage/$1.warp"
	got=$("$warp" "$stage/$1.warp" 2>&1)
	if [ "$got" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: $got"; failures=$((failures + 1)); fi
}
check sound_used $'use sound\n1' 1
check note_shown $'use sound\nprint(note(440))' A4
check note_beside_quantity $'use sound\nprint(note(440).text() + " " + (2 m).text())' "A4 2m"
exit $((failures > 0))
