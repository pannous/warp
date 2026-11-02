#!/usr/bin/env bash
# `@primitive` marks a whole file anywhere in its leading header, the `#!` line, comments and blank lines before the
# code; on its own line below the code it is refused (card primitive-comment-header). Run it with
# bash test/native/test_primitive_header.sh; the warp under test is WARP (default the warp on PATH)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
FIB='fib(n: i64): i64 := if n < 2 then n else fib(n - 1) + fib(n - 2)
fib 20'

printf '#!/usr/bin/env warp\n# a comment before the mark\n\n@primitive\n%s\n' "$FIB" >"$stage/commented.warp"
check "the mark below header comments" "$("$warp" "$stage/commented.warp" 2>&1 | tail -1)" 6765
printf '@primitive\n%s\n' "$FIB" >"$stage/first.warp"
check "the mark on the first line" "$("$warp" "$stage/first.warp" 2>&1 | tail -1)" 6765
printf '# a comment\nx = 1\n@primitive\n%s\n' "$FIB" >"$stage/late.warp"
check "the mark below the code refused" "$("$warp" "$stage/late.warp" 2>&1 | grep -c 'marks a file only in its leading header')" 1
exit $((failures > 0))
