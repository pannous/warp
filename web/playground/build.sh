#!/bin/bash
# Builds web/playground: the warp compiler as warp.wasm (no wasmtime: the `native` feature off, src/web.rs) and
# samples.js (samples/*.wasp for the example menu). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/
set -euo pipefail

STACK_BYTES=8388608 # the parser and the emitter recurse deeply; wasm32's default stack is 1 MB

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

target_dir="$(cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
RUSTFLAGS="-C link-arg=-zstack-size=$STACK_BYTES" cargo rustc --offline --lib --crate-type cdylib --release \
	--target wasm32-unknown-unknown --no-default-features
compiled="$target_dir/wasm32-unknown-unknown/release/warp.wasm"
if command -v wasm-opt > /dev/null; then
	wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals "$compiled" -o "$page/warp.wasm"
else
	cp "$compiled" "$page/warp.wasm"
fi

python3 - "$page/samples.js" samples/*.wasp <<'PYTHON'
import json, os, sys
samples = {os.path.basename(path)[:-len(".wasp")]: open(path, encoding="utf-8").read() for path in sys.argv[2:]}
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from samples/*.wasp\nconst SAMPLES = " + json.dumps(samples, ensure_ascii=False, indent="\t") + ";\n")
PYTHON
echo "built $page/warp.wasm ($(wc -c < "$page/warp.wasm") bytes) and samples.js"
