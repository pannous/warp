#!/usr/bin/env bash
# A pairs line that reads as no case, an unescaped ' opening a text over the next lines, is a failure naming its line,
# never silently dropped with the lines it swallows (card core-pairs-quote-drop). Run it with
# bash test/native/test_unread_pair_lines.sh; the warp under test is WARP (default the warp on PATH), its compiler from
# WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
cases=$(cd "$(dirname "$0")/../.." && pwd)/data/unread-pairs
mkdir -p "$cases"
printf "'1 + 1'\t2\n'\"I'm\"'\t\"I'm\"\n'2 + 2'\t4\n" >"$cases/test_quote.pairs.warp"
said=$("$warp" test "$cases" 2>&1); status=$?
status_of_test=0
expect() {
	case $said in
		*"$1"*) echo "✓ $1" ;;
		*) echo "✗ expected …$1…, got: $said"; status_of_test=1 ;;
	esac
}
expect "✗ $cases/test_quote.pairs.warp:2"
expect "✗ $cases/test_quote.pairs.warp:3"
expect "reads as no test pair"
expect "2 of 3 failed in 1 files"
[ $status -ne 0 ] && echo "✓ exit $status" || { echo "✗ exit 0"; status_of_test=1; }
exit $status_of_test
