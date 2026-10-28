#!/usr/bin/env bash
# scripts/wasmtime-c-api.sh [<folder>]: warp's wasmtime C API (crates/wasmtime-c-api, card wasmtime-bundled) built for this
# machine into /opt/cargo, and copied into <folder>, default beside the warp binary on PATH, where the native warp and
# built executables find it by rpath. CI builds the release's per-platform assets (.github/workflows/wasmtime-c-api.yml)
set -eu -o pipefail
checkout=$(cd "$(dirname "$0")/.." && pwd)
folder=${1:-$(dirname "$(command -v warp)")}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/opt/cargo/warp-wasmtime}
(cd "$checkout/crates/wasmtime-c-api" && nice -n 19 cargo build --release --locked)
case $(uname -s) in
  Darwin) library=libwarp_wasmtime.dylib ;;
  Linux) library=libwarp_wasmtime.so ;;
  *) library=warp_wasmtime.dll ;;
esac
cp "$CARGO_TARGET_DIR/release/$library" "$folder/$library"
echo "wasmtime-c-api: $folder/$library"
