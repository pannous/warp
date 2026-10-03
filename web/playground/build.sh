#!/bin/bash
# Builds web/playground: the warp compiler for the browser (no wasmtime: the `native` feature off, src/web.rs) in two
# builds, and samples.js (samples/*.wasp for the example menu). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/  (?debug: debug build)
# Usage: build.sh [optimized|debug]   (both when omitted)
#   optimized → warp.wasm: release profile (opt-level z, fat LTO, one codegen unit, stripped), without the `validate`
#               feature (wasmparser's validator, a quarter of the module: the browser validates anyway), then wasm-opt
#   debug     → warp.debug.wasm: profile web-debug (opt-level 1, line tables, the name section kept), with `validate`, so
#               emitter bugs are named and stack traces and the browser's debugger show Rust functions and lines
# Measured 2026-10-03: optimized 1.30 MB (545 KB gzipped); debug 22 MB (4.8 MB gzipped; full DWARF would be 53 MB).
# Not taken: -Cpanic=immediate-abort with -Zbuild-std saves another 6% but needs nightly and loses the panic messages.
set -euo pipefail

STACK_BYTES=8388608 # the parser and the emitter recurse deeply; wasm32's default stack is 1 MB
WASM_OPT_FLAGS=(-Oz --converge --strip-debug --strip-producers
	--enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals)

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

offline=$([ "${CARGO_NET_OFFLINE:-true}" = false ] || echo --offline) # CI (.github/workflows/pages.yml) fetches crates
target_dir="$(cargo metadata $offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

# compile the cdylib with a cargo profile and features; prints the path of the module
compile() {
	local profile=$1; shift
	RUSTFLAGS="-C link-arg=-zstack-size=$STACK_BYTES" cargo rustc $offline --lib --crate-type cdylib --profile "$profile" \
		--target wasm32-unknown-unknown --no-default-features "$@" >&2
	echo "$target_dir/wasm32-unknown-unknown/$profile/warp.wasm"
}

build_optimized() {
	local compiled
	compiled=$(compile release)
	# without wasm-opt (or an older one refusing a feature) the module of the LTO build serves, a seventh larger
	wasm-opt "${WASM_OPT_FLAGS[@]}" "$compiled" -o "$page/warp.wasm" || cp "$compiled" "$page/warp.wasm"
	echo "built $page/warp.wasm ($(wc -c < "$page/warp.wasm") bytes)"
}

build_debug() {
	cp "$(compile web-debug --features validate)" "$page/warp.debug.wasm"
	echo "built $page/warp.debug.wasm ($(wc -c < "$page/warp.debug.wasm") bytes)"
}

case "${1:-both}" in
	optimized) build_optimized ;;
	debug) build_debug ;;
	both) build_optimized; build_debug ;;
	*) echo "usage: $0 [optimized|debug]" >&2; exit 2 ;;
esac

python3 - "$page/samples.js" samples/*.wasp <<'PYTHON'
import json, os, sys
samples = {os.path.basename(path)[:-len(".wasp")]: open(path, encoding="utf-8").read() for path in sys.argv[2:]}
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from samples/*.wasp\nconst SAMPLES = " + json.dumps(samples, ensure_ascii=False, indent="\t") + ";\n")
PYTHON
echo "built $page/samples.js"
