#!/usr/bin/env bash
# scripts/wasmtime-c-api.sh [<folder>…]: warp's wasmtime C API (crates/wasmtime-capi, card wasmtime-bundled) built for
# this machine into /opt/cargo (a no-op when current), and copied into each folder, default the one beside the warp
# binary on PATH: after-push.sh installs it beside warp and into the main checkout's lib/, rebuild.sh into a tree's lib/,
# the library folder a native build links it from and its executables find it in (rpath)
set -eu -o pipefail
checkout=$(cd "$(dirname "$0")/.." && pwd)
[ $# -gt 0 ] || set -- "$(dirname "$(readlink -f "$(command -v warp)")")"
export CARGO_TARGET_DIR=${WARP_WASMTIME_TARGET:-/opt/cargo/warp-wasmtime}
(cd "$checkout/crates/wasmtime-capi" && nice -n 19 cargo build --release --locked -q)
case $(uname -s) in
  Darwin) library=libwarp_wasmtime.dylib ;;
  Linux) library=libwarp_wasmtime.so ;;
  *) library=warp_wasmtime.dll ;;
esac
for folder in "$@"; do
  cmp -s "$CARGO_TARGET_DIR/release/$library" "$folder/$library" && continue
  # moved into place whole: a run loading it meanwhile meets the old one or the new one
  cp "$CARGO_TARGET_DIR/release/$library" "$folder/$library.new" && mv -f "$folder/$library.new" "$folder/$library"
  echo "wasmtime-c-api: $folder/$library"
done
