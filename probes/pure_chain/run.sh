#!/bin/bash
# the parser milestone: parser.warp and the passes compiled by the pure lane read <program.warp> into its lowered view,
# emitter.warp (Rust-compiled, as the emit stage runs today) makes the module, which runs: run.sh [program.warp]
here=$(cd "$(dirname "$0")" && pwd)
program=$(cd "$(dirname "${1:-$here/hello.warp}")" && pwd)/$(basename "${1:-$here/hello.warp}")
out=$here/../../data/pure_chain; mkdir -p "$out"
export WARP_LIB="$here/../../lib" WARP_HINTS=0 WARP_NO_WINDOW=1 WARP_TIME_LIMIT=900
warp=/Users/me/dev/bin/warp
cd "$out" || exit 1
cp "$here/view.warp" view.warp && sed -i '' "s#\.\./\.\./lib#$here/../../lib#" view.warp
WARP_SELF_HOSTED=pure-only $warp compile --wasm view.warp >/dev/null 2>&1 || { echo "pure compile of view.warp failed"; exit 1; }
cp "$program" program.warp
$warp view.wasm > view.json || { cat view.json; exit 1; }
# the runtime words: runtime.warp with the files its `use` lines name after it, lowered (self_hosted.rs runtime_words)
compiler="$here/../../lib/compiler"
{ grep -v '^use ' "$compiler/runtime.warp"; for used in $(grep '^use ' "$compiler/runtime.warp" | sed 's/^use //; s/"//g'); do
	if [ -f "$compiler/$used" ]; then cat "$compiler/$used"; else cat "$compiler/../$used.warp"; fi; echo
done; } > runtime.warp
$warp lower --json "$(cat runtime.warp)" > runtime.json || exit 1
$warp "$here/emit.warp" "$out/view.json" "$out/runtime.json" "$out/program.wasm" 2>&1 | grep -v "^warning\|^  assumed\|^ *[0-9]* |\|^ *|"
$warp program.wasm
