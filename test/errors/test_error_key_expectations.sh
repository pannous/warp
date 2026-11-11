#!/usr/bin/env bash
# a pairs line expecting `error(<key>)` passes only on the compile error of that row of core/analysis/errors.data, never
# on another error; the key leads the error line only for `warp test` (--value-as-data), never in what a person sees
# (card error-kind-expectations). Run it with bash test/errors/test_error_key_expectations.sh; the warp is WARP (default
# the warp on PATH), its lib LIB (default its own; a tree's: LIB=<tree>/lib)
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
export WARP_WINDOW=0
failures=0
cases=$(mktemp -d "${TMPDIR:-/tmp}/error-key.XXXXXX")
trap 'rm -rf "$cases"' EXIT
charged="'x := 1; x = 2; x'"

# judged <expected> <exit wanted>: a pairs file of the one charged-assignment case expecting it, run by warp test
judged() {
	printf '%s\t%s\n' "$charged" "$1" >"$cases/test_case.pairs.warp"
	"$warp" --lib "$lib" test "$cases/test_case.pairs.warp" >"$cases/said" 2>&1
	local status=$?
	if [ $status = "$2" ]; then echo "✓ $1 exits $2"; else echo "✗ $1 exits $status, not $2: $(tail -3 "$cases/said")"; failures=$((failures + 1)); fi
}
judged 'error(charged_assignment)' 0
judged 'error(kind_change)' 1
said=$("$warp" --lib "$lib" "x := 1; x = 2; x" 2>&1)
if [[ $said == *"error(charged_assignment)"* ]]; then echo "✗ a person sees the key: $said"; failures=$((failures + 1)); else echo "✓ no key shown"; fi
[ $failures = 0 ]
