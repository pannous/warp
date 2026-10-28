#!/usr/bin/env bash
# core/native/warp.warp, the native warp (decision native-warp-in-core), built by `warp build native`: it runs a file
# with its arguments, a module, code, and warp.wasm's own commands (`warp build`). Run it with
# bash test/native/test_warp_native.sh; the warp that builds it is WARP (default the warp on PATH), the native warp's
# lib/ WARP_LIB as usual
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}

cp "$checkout/core/native/driver.warp" "$checkout/core/native/warp.warp" "$checkout/samples/hello/hello.warp" "$stage/"
(cd "$stage" && "$warp" build native warp.warp >"$stage/build.log" 2>&1)
if [ -x "$stage/warp" ]; then
	native() { (cd "$stage" && ./warp "$@" 2>&1); }
	check "a file" "$(native hello.warp)" "Hello 🌍2026"
	printf 'use os\nprint args\n' >"$stage/arguments.warp"
	check "a file's arguments" "$(native arguments.warp eins zwei)" '["eins" "zwei"]'
	(cd "$stage" && "$warp" build --wasm hello.warp >/dev/null 2>&1)
	check "a module" "$(native hello.wasm)" "Hello 🌍2026"
	check "code" "$(native '3 + 3')" 6
	check "help, warp.wasm's command" "$(native help | head -1)" "warp, the compiler written in warp (core/compiler.warp), its commands:"
	mkdir "$stage/built" && cp "$stage/hello.warp" "$stage/built/"
	(cd "$stage/built" && ../warp build hello.warp >/dev/null 2>&1)
	check "warp build by the native warp" "$("$stage/built/hello" 2>&1)" "Hello 🌍2026"
else
	echo "✗ warp: no executable"; head -5 "$stage/build.log"; failures=$((failures + 1))
fi
exit $((failures > 0))
