#!/usr/bin/env bash
# the native warp's options before or after its command word (wiki/cli.md, wiki/warning.md: `--no-hints`), and
# `warp version` saying when it was built with no PATH (no `date` to run). Run it with
# bash test/native/test_cli_options.sh; the warp that builds it is WARP (default the warp on PATH), the native warp's
# lib/ WARP_LIB as usual
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
unset WARP_HINTS
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}

cp "$checkout"/core/native/*.warp "$stage/"
(cd "$stage" && "$warp" build native warp.warp >"$stage/build.log" 2>&1)
[ -x "$stage/warp" ] || { echo "✗ warp build native warp.warp: $(tail -3 "$stage/build.log")"; exit 1; }
native() { (cd "$stage" && ./warp "$@" 2>&1); }
# a word of a standard module named without its use is a hint (`fix: use sound`)
printf 'print A4\n' >"$stage/hinted.warp"
check "hints by default" "$(native run hinted.warp | grep -c '^hint')" 1
check "--no-hints before run" "$(native --no-hints run hinted.warp)" 440
check "--no-hints after run" "$(native run --no-hints hinted.warp)" 440
check "--no-hints before the file" "$(native --no-hints hinted.warp)" 440
check "version's build time with no PATH" "$(cd "$stage" && env PATH= ./warp version 2>&1 | sed -n 's/^the native warp · built //p' | grep -cE '^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}$')" 1
[ $failures = 0 ]
