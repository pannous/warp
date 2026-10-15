#!/bin/bash
# Probe for card create-little (reviewer): warp.wasm's size as the playground builds it (build.sh build_optimized), for
# checkouts given as arguments, to see what naga's GLSL front end and WGSL writer add. Usage: wasm_size.sh <checkout>…
set -euo pipefail
for checkout in "$@"; do
	cd "$checkout"
	target_dir="$(cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
	RUSTFLAGS="-C link-arg=-zstack-size=8388608" cargo rustc --offline --lib --crate-type cdylib --profile release \
		--target wasm32-unknown-unknown --no-default-features >/dev/null 2>&1
	compiled="$target_dir/wasm32-unknown-unknown/release/warp.wasm"
	optimized="$checkout/scratch/warp.size.wasm"
	wasm-opt -Oz --converge --strip-debug --strip-producers --enable-bulk-memory --enable-nontrapping-float-to-int \
		--enable-sign-ext --enable-mutable-globals "$compiled" -o "$optimized"
	echo "$(basename "$checkout"): $(wc -c <"$compiled") bytes built, $(wc -c <"$optimized") after wasm-opt, $(gzip -9c "$optimized" | wc -c) gzipped"
done
