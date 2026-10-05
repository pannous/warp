#!/bin/bash
# Prove two-Module + one Store via Linker shares a tiny runtime (g-qV-s first slice).
# No component model; import module name is warp_runtime. Run from the repo root.
# usage: probes/aot/shared_runtime.sh
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
SCRATCH=$ROOT/scratch/aot/shared_runtime_build
mkdir -p "$SCRATCH" scratch/aot/shared_runtime
PROBE=$ROOT/probes/aot/shared_runtime_linker.rs

# Ad-hoc crate under scratch/ (probes/ forbids Cargo.toml and src/ paths)
cat > "$SCRATCH/Cargo.toml" <<TOML
# Empty workspace table: keep this scratch crate out of the repo workspace.
[workspace]

[package]
name = "shared-runtime-linker"
version = "0.0.0"
edition = "2021"
publish = false

[[bin]]
name = "shared_runtime_linker"
path = "$PROBE"

[dependencies]
wasmtime = { version = "49.0.1", default-features = false, features = ["anyhow", "cranelift", "runtime", "gc", "gc-copying", "wat", "std"] }

[profile.dev]
debug = "line-tables-only"
incremental = false
TOML

# Reuse the workspace lock / registry; stay offline like the rest of the tree.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export WARP_SCRATCH="$ROOT/scratch"
(cd "$SCRATCH" && cargo run --offline -q)
echo "ok shared_runtime"
