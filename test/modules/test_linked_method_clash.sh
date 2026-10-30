#!/usr/bin/env bash
# A file's class method beside a linked module's class declaring the same method (reals' Real *, units' Quantity text)
# runs as the program's own (card file-use): a file run links the prebuilt modules, a test pair does not. Run it with
# bash test/modules/test_linked_method_clash.sh; the warp under test is WARP (default the warp on PATH)
warp=${WARP:-warp}
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
check() { # check <name> <program> <expected output>
	printf '%s\n' "$2" >"$stage/$1.warp"
	got=$("$warp" "$stage/$1.warp" 2>&1)
	if [ "$got" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: $got"; failures=$((failures + 1)); fi
}
check operator_beside_real $'use math\nclass V {\n\ta: number\n\t*(other) := V(a * other)\n}\nprint((V(2) * 3).a)' 6
check units_after_math $'use math\nuse units\nx = 6 m\nprint(x)' 6m
check text_beside_real $'use math\nuse units\nprint(m.text())' 1m
exit $((failures > 0))
