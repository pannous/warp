#!/usr/bin/env bash
# The console in pure warp (lib/repl.warp, card pure-repl) on warp-host: entries piped to its standard input, what it
# prints checked. Run it through the queue: tests/queue.sh tests/host/test_repl.sh
# The host and compilers are found as tests/host/test_host.sh finds them: WARP_HOST_BINARY (default the checkout's
# target/host/warp-host), compiler-self.wasm beside WARP_LAUNCHER_RUST (default target/debug/warp)
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/repl-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$(dirname "$rust")/compiler-self.wasm" ] || { echo "✗ no compiler-self.wasm beside $rust: $rust rebuild builds it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$(dirname "$rust")/compiler-self.wasm" "$stage/bin/" || exit 1
cd "$stage" || exit 1

run=0 failed=0
# console <name> <text the output holds>… [! <text it lacks>…] -- <entries…>: the entries one per line on stdin
console() {
	local name=$1 wanted=() unwanted=() said text problems=""
	shift
	while [ "$1" != -- ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	shift
	said=$(printf '%s\n' "$@" | "$stage/bin/warp-host" "$checkout/lib/repl.warp" 2>&1)
	local status=$?
	run=$((run + 1))
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	for text in "${unwanted[@]}"; do [[ $said == *"$text"* ]] && problems+=" unwanted [$text]"; done
	if [ "$status" = 0 ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $status,$problems"
		echo "$said" | sed 's/^/    /'
	fi
}

console "an entry's value is printed" "Interactive console" "🌀42" -- '6*7'
console "an assignment stays for later entries" "🌀7"$'\n'"🌀49" -- 'a = 7' 'a * a'
console "a definition stays, its value not shown" "🌀🌀49" -- 'square(x) := x * x' 'square(7)'
console "a use stays" "🌀🌀['x']" -- 'use os' 'split("x", " ")'
console "++ changes the kept value" "🌀8" -- 'a = 7' 'a++' 'a'
console "an expression is not replayed" "🌀printed once"$'\n'"🌀1" ! "printed once"$'\n'"printed once" \
	-- 'print "printed once"' '1'
console "open brackets read on" "… … 1"$'\n'"2" -- 'for i in [1 2] {' ' print i' '}'
console "an unknown name is an Error" "Error: " "unknown name nope" -- 'nope + 1'
console "a raise is an Error, the console goes on" "Error: boom" "🌀3" -- 'raise "boom"' '1 + 2'
console "a failed assignment is not kept" "unknown name b" -- 'b = nope + 1' 'b'
console ":reset forgets the session" "unknown name a" -- 'a = 7' ':reset' 'a'
console ":session lists the kept entries" "a = 7" -- 'a = 7' 'a' ':session'
console "quit ends the console" ! "42" -- 'quit' '6*7'
console "the input's end ends the console" "🌀1" -- '1'

# the session keeps an assignment's value, not its effects: later entries never run them again
console "an assignment's effect runs once" "🌀🌀once"$'\n'"🌀once"$'\n'"🌀once!" ! "once"$'\n'"once" \
	-- 'use os' 'x = command("echo once >&2; echo once")' 'x' 'x + "!"'
console "a kept assignment is its value" 'x = "a \"q\""' "n = 8" -- 'x = "a \"q\""' 'n = 7' 'n++' ':session'
console "an Error names the entry's line, not the scratch file" "unknown name nope at line 1:1" ! "/tmp/" \
	! "os.compile_wasm" -- 'a = 1' 'nope'

if [ "$failed" = 0 ]; then echo "✓ $run tests passed"; else echo "✗ $failed of $run tests failed"; exit 1; fi
