#!/bin/bash
# Rebuild ../rust_demo.wasm, the WebAssembly component of this Rust crate (tests/ffi/test_components.rs):
# wasm32-wasip2 makes a component directly, wit-bindgen exports the WIT world in wit/demo.wit
set -e
cd "$(dirname "$0")"
cargo build --config net.offline=false --release --target wasm32-wasip2
TARGET=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
cp "$TARGET/wasm32-wasip2/release/warp_component_demo.wasm" ../rust_demo.wasm
wasm-tools component wit ../rust_demo.wasm | grep -v "^  import" | head -5
