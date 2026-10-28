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
	check "run, a file" "$(native run hello.warp)" "Hello 🌍2026"
	check "the arguments after --" "$(native arguments.warp -- eins zwei)" '["eins" "zwei"]'
	check "self, code by warp-self.wasm" "$(native self '3 + 4')" 7
	printf 'raise "boom"\n' >"$stage/raising.warp"
	check "what a program raised" "$(native raising.warp)" "boom"
	check "the console, its lines staying" "$(printf 'a = 7\nx = [1\na * 6\n' | native repl | sed -n 4p)" "🌀42"
	check "bare warp, a piped program" "$(printf 'a = 7\na * 6\n' | native)" 42
	check "exit(code) ends the run" "$(native 'exit(3)'; echo $?)" 3
	printf 'x  =  1\n' >"$stage/spaced.warp"
	check "format check" "$(native format check spaced.warp | head -1)" "spaced.warp:1: layout: spaces"
	check "eval, code" "$(native eval '3 * 4')" 12
	check "a command the native warp lacks yet" "$(native verify 1)" "warp verify is not in the native warp yet: warp-rust verify runs it"
	mkdir "$stage/below"
	check "a file above the current folder" "$(cd "$stage/below" && ../warp ../hello.warp 2>&1)" "Hello 🌍2026"
	check "a command's path above the current folder" "$(cd "$stage/below" && ../warp compile ../hello.warp ../above.wasm >/dev/null 2>&1; ls "$stage/above.wasm" 2>&1)" "$stage/above.wasm"
	printf 'i = 0\nwhile yes { i += 1 }\n' >"$stage/endless.warp"
	check "fuel, out of it" "$(native fuel 1_000_000 endless.warp | cut -c1-31)" "out of fuel after 1000000 steps"
	check "rebuild of no compiler" "$(native rebuild nonsense | cut -c1-34)" "warp rebuild: no compiler nonsense"
	check "sleep_ms and clock" "$(native 'c = clock(); sleep_ms(50); clock() - c >= 50')" yes
	check "random, the same after the same seed" "$(native 'random_seed(7); a = random(); random_seed(7); a == random() and a < 1')" yes
	check "regex replace with groups" "$(native 'use regex; replace_all("john smith", "(\\w+) (\\w+)", "$2 $1")')" "smith john"
	check "regex all" "$(native 'use regex; find_all("x12y345", "[0-9]+")')" '["12" "345"]'
	check "sha256" "$(native 'use hash; sha256("abc")')" ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
	check "crc32" "$(native 'use hash; crc32("abc")')" 891568578
	check "a system value of a library's" "$(native 'battery()')" "the system value battery is not in the native warp yet (warp-rust has it)"
else
	echo "✗ warp: no executable"; head -5 "$stage/build.log"; failures=$((failures + 1))
fi
exit $((failures > 0))
