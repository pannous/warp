#!/bin/bash
# the parser milestone: parser.warp and the passes compiled by the pure lane read <program.warp> into its lowered view,
# emitter.warp (Rust-compiled, as the emit stage runs today) makes the module, which runs: run.sh [program.warp]
# view.wasm and runtime.json depend only on lib/ and the warp binary: cached under data/pure_chain/cache/<their hash>
set -o pipefail
here=$(cd "$(dirname "$0")" && pwd)
program=$(cd "$(dirname "${1:-$here/hello.warp}")" && pwd)/$(basename "${1:-$here/hello.warp}")
lib=$(cd "$here/../../lib" && pwd)
export WARP_LIB="$lib" WARP_HINTS=0 WARP_NO_WINDOW=1 WARP_TIME_LIMIT=900
warp=/Users/me/dev/bin/warp
compiler="$lib/compiler"
inputs_hash=$({ shasum "$(readlink -f $warp)" "$here/view.warp"; find "$lib" -name '*.warp' -exec shasum {} +; } | sort | shasum | cut -c1-16)
cache=$here/../../data/pure_chain/cache/$inputs_hash
run=$here/../../data/pure_chain/run-$$
mkdir -p "$run" && trap 'rm -rf "$run"' EXIT
fail() { echo "pure chain: $1 failed" >&2; exit 1; }
if [ ! -f "$cache/runtime.json" ]; then
	mkdir -p "$cache.part" && cd "$cache.part" || exit 1
	sed "s#\.\./\.\./lib#$lib#" "$here/view.warp" > view.warp
	WARP_SELF_HOSTED=pure-only $warp compile --wasm view.warp >/dev/null 2>&1 || fail "pure compile of view.warp"
	# the runtime words: runtime.warp with the files its `use` lines name after it, lowered (self_hosted.rs runtime_words)
	{ grep -v '^use ' "$compiler/runtime.warp"; for used in $(grep '^use ' "$compiler/runtime.warp" | sed 's/^use //; s/"//g'); do
		if [ -f "$compiler/$used" ]; then cat "$compiler/$used"; else cat "$compiler/../$used.warp"; fi; echo
	done; } > runtime.warp
	$warp lower --json "$(cat runtime.warp)" > runtime.json || fail "lowering runtime.warp"
	rm -rf "$cache" && mv "$cache.part" "$cache"
fi
cd "$run" || exit 1
cp "$program" program.warp
$warp "$cache/view.wasm" > view.json || { cat view.json; fail "the pure-compiled parser and passes"; }
# from a folder that stays the same: the working directory is part of the compiled program's cache key (program_cache.rs)
(cd "$here" && $warp emit.warp "$run/view.json" "$cache/runtime.json" "$run/program.wasm") 2>&1 | grep -v "^warning\|^  assumed\|^ *[0-9]* |\|^ *|"
[ -s program.wasm ] || fail "emitter.warp"
$warp program.wasm
