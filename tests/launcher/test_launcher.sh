#!/usr/bin/env bash
# What the launcher packaging/bin/warp prints and exits with (notes/pure_wrapper.md): programs through the pure
# pipeline, other commands through warp-rust. Run it through the queue: tests/queue.sh tests/launcher/test_launcher.sh
# The Rust binary under test is WARP_LAUNCHER_RUST (default the checkout's target/debug/warp) with compiler.wasm and
# compiler-self.wasm beside it (`warp rebuild pure`, `warp rebuild self`); it and the launcher are staged together in
# scratch/launcher-test/, as installed.
checkout=$(cd "$(dirname "$0")/../.." && pwd)
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
stage=$checkout/scratch/launcher-test
hello=$checkout/samples/hello/hello.warp
HELLO='"Hello 🌍2026"'
# a form the pure pipeline refuses today; once it compiles, another refused one takes its place
REFUSED='3 times { print 1 }'
REFUSAL='outside the core'
export WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_SELF_HOSTED

for compiler in compiler.wasm compiler-self.wasm; do
	[ -f "$(dirname "$rust")/$compiler" ] || { echo "✗ no $compiler beside $rust: $rust rebuild builds it"; exit 1; }
done
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/elsewhere" || exit 1
ln -f "$rust" "$stage/bin/warp-rust" 2>/dev/null || cp "$rust" "$stage/bin/warp-rust" || exit 1
cp "$(dirname "$rust")"/compiler*.wasm "$checkout/packaging/bin/warp" "$stage/bin/" || exit 1
cp "$hello" "$stage/hello.warp" && echo "$REFUSED" >"$stage/refused.warp" || exit 1
# reached through a link, as Homebrew links bin/warp: warp-rust is found beside the link's target
ln -s "$stage/bin/warp" "$stage/elsewhere/warp"
warp=$stage/elsewhere/warp

run=0 failed=0
# expect <name> <exit status> <text the output holds>… -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() said got
	shift 2
	while [ "$1" != -- ]; do wanted+=("$1"); shift; done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	local text missing=""
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || missing+=" [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$missing" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status), missing:$missing"
		echo "$said" | sed 's/^/    /'
	fi
}

expect "a program file runs through the pure pipeline" 0 "$HELLO" -- "$warp" "$hello"
expect "warp run" 0 "$HELLO" -- "$warp" run "$hello"
expect "--self: compiler-self.wasm" 0 "$HELLO" -- "$warp" --self "$hello"
expect "WARP_SELF_HOSTED=1: compiler-self.wasm" 0 "$HELLO" -- env WARP_SELF_HOSTED=1 "$warp" "$hello"
expect "inline code" 0 "inline 42" -- "$warp" 'print "inline " + 6*7'
expect "a refused program fails loudly" 1 "$REFUSAL" "refused.warp" -- "$warp" "$stage/refused.warp"
expect "refused inline code fails loudly" 1 "$REFUSAL" "at 1:1" -- "$warp" "$REFUSED"
expect "WARP_FALLBACK=rust: a refused program runs through Rust, said loudly" 0 \
	"THE PURE PIPELINE FAILED" "RUNNING THROUGH RUST" $'1\n1\n1' -- env WARP_FALLBACK=rust "$warp" "$REFUSED"
expect "WARP_PIPELINE=rust: the program never meets the pure pipeline" 0 $'1\n1\n1' -- \
	env WARP_PIPELINE=rust "$warp" "$REFUSED"
expect "compile --wasm writes the module" 0 "bytes to $stage/hello.wasm" -- "$warp" compile --wasm "$stage/hello.warp"
expect "the compiled module runs" 0 "$HELLO" -- "$warp" "$stage/hello.wasm"
expect "a refused compile fails loudly" 1 "$REFUSAL" -- "$warp" compile --wasm "$stage/refused.warp"
expect "other commands are warp-rust's: version" 0 "🌀 Warp" -- "$warp" version
expect "other commands are warp-rust's: help" 0 "warp pure <file.warp>" -- "$warp" --help

if [ $failed -gt 0 ]; then echo "$failed of $run failed"; exit 1; fi
echo "✓ $run tests passed"
