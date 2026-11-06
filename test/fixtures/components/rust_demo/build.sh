#!/bin/bash
# Rebuild the two fixtures of this Rust crate (src/lib.rs): ../rust_demo.wasm, a core module (wasm32-unknown-unknown)
# that `use x.wasm` links, and ../rust_demo.component.wasm, the WebAssembly component of wit/demo.wit (wasm32-wasip2,
# feature component), kept for when warp links components (decision: core module first)
set -e
cd "$(dirname "$0")"
TARGET=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
built() { cargo build --config net.offline=false --release --target "$1" "${@:3}" && cp "$TARGET/$1/release/warp_component_demo.wasm" "../$2"; }
built wasm32-unknown-unknown rust_demo.wasm
built wasm32-wasip2 rust_demo.component.wasm --features component
wasm-tools print ../rust_demo.wasm | grep "(export"
wasm-tools component wit ../rust_demo.component.wasm | grep -v "^  import" | head -5
