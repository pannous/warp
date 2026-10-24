#!/bin/bash
# scripts/cloud-test.sh drops a warm VM's compiler.wasm / compiler-self.wasm when lib/ or the Rust sources changed since
# the last job, and keeps them otherwise (card cloud-stale-compiler). Works in scratch/stale-compilers/, a git repo of its own
set -u
script=$(cd "$(dirname "$0")/.." && pwd)/scripts/cloud-test.sh
eval "$(sed '$d' "$script")"
HERE_ROOT=$(cd "$(dirname "$0")/.." && pwd)/scratch/stale-compilers
mkdir -p "$HERE_ROOT/tree" && cd "$HERE_ROOT/tree" || exit 1
[ -d .git ] || git init -q
mkdir -p lib src crates target/tmp data
echo "byte_codes $(date +%s%N)" > lib/a.warp; echo fn > src/a.rs; echo c > crates/a; echo b > build.rs; echo l > Cargo.lock
git add -A && git -c user.name=probe -c user.email=probe@example.com commit -qm one
build_files() { touch target/tmp/compiler-self.wasm target/compiler.wasm data/compiler-self.wasm; }
build_files; drop_stale_compilers                         # cloud-test: …: dropped 3 compiler build files
build_files; drop_stale_compilers; echo "kept: $(ls target/tmp)"   # kept: compiler-self.wasm
echo more >> lib/a.warp; git commit -qam two
drop_stale_compilers                                      # cloud-test: …: dropped 3 compiler build files
