#!/bin/bash
# Build a standalone executable of a warp program (probes/aot/standalone_runner.rs) and report its size and startup.
# usage: probes/aot/standalone.sh <program.wasp> [warp binary]     (run from the repo root)
set -e
SOURCE=$1
WARP=${2:-scratch/warp}
CRATE=scratch/aot/standalone
mkdir -p "$CRATE/src"
cp probes/aot/standalone_runner.rs "$CRATE/src/main.rs"
cat > "$CRATE/Cargo.toml" <<TOML
[package]
name = "warp-standalone"
version = "0.1.0"
edition = "2021"
[workspace]
[dependencies]
wasmtime = { version = "49.0.1", default-features = false, features = ["runtime", "gc", "gc-copying", "std"] }
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
TOML
"$WARP" compile --aot "$SOURCE" >/dev/null
export WARP_CWASM=$(cd "$(dirname "$SOURCE")" && pwd)/$(basename "${SOURCE%.*}").cwasm
TARGET=$(cd "$CRATE" && cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
(cd "$CRATE" && cargo build --offline --release 2>&1 | tail -1)
BINARY=$TARGET/release/warp-standalone
cp "$BINARY" "${SOURCE%.*}.standalone"
echo "cwasm $(stat -f %z "$WARP_CWASM") B, executable $(stat -f %z "${SOURCE%.*}.standalone") B"
for _ in 1 2 3; do /usr/bin/time -p "${SOURCE%.*}.standalone" 2>&1 | grep -E "result|real"; done
