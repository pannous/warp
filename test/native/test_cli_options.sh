#!/usr/bin/env bash
# the native warp's options before or after its command word (wiki/cli.md, wiki/warning.md: `--no-hints`), and
# `warp version` saying when it was built with no PATH (no `date` to run). Run it with
# bash test/native/test_cli_options.sh; the warp that builds it is WARP (default the warp on PATH); a tree's:
# WARP=<tree>/data/gate/warp
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
# the lib/ that warp runs with (its version names it), given to the native warp built here
library=$("$warp" version 2>/dev/null | sed -n 's/.*(compiler.warp of \(.*\))$/\1/p')
export WARP_WINDOW=0
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}

cp "$checkout"/core/native/*.warp "$stage/"
(cd "$stage" && "$warp" build native warp.warp >"$stage/build.log" 2>&1)
[ -x "$stage/warp" ] || { echo "✗ warp build native warp.warp: $(tail -3 "$stage/build.log")"; exit 1; }
native() { (cd "$stage" && ./warp --lib "$library" "$@" 2>&1); }
# a word of a standard module named without its use is a hint (`fix: use sound`)
printf 'print A4\n' >"$stage/hinted.warp"
check "hints by default" "$(native run hinted.warp | grep -c '^hint')" 1
check "--no-hints before run" "$(native --no-hints run hinted.warp)" 440
check "--no-hints after run" "$(native run --no-hints hinted.warp)" 440
check "--no-hints before the file" "$(native --no-hints hinted.warp)" 440
# an option before a command word run by a lib/ program (format, app, dev) stays an option: format counts one file
printf 'x=1\n' >"$stage/unformatted.warp"
check "--no-hints before format" "$(native --no-hints format unformatted.warp </dev/null | grep -o '^warp format: [0-9]* files')" "warp format: 1 files"
check "version's build time with no PATH" "$(cd "$stage" && env PATH= ./warp --lib "$library" version 2>&1 | sed -n 's/^the native warp · built //p' | grep -cE '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}$')" 1
# --fast drops the run's runtime checks (decision fast-mode): an int past 64 bits wraps, the written and the boxed one
# alike; without it the int grows, and a value already big keeps its exact arithmetic either way
printf '9223372036854775807 + 1\n' >"$stage/past.warp"
printf 'xs = [9223372036854775807]\nxs#1 + 1\n' >"$stage/boxed.warp"
printf '18446744073709551616 + 1\n' >"$stage/big.warp"
check "--fast wraps a written int" "$(native --fast past.warp)" -9223372036854775808
check "--fast wraps a boxed int" "$(native --fast boxed.warp)" -9223372036854775808
check "without --fast it grows" "$(native boxed.warp)" 9223372036854775808
check "--fast keeps a big value exact" "$(native --fast big.warp)" 18446744073709551617
[ $failures = 0 ]
