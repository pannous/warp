#!/usr/bin/env bash
# packaging/native_dist.sh <dist>: a release folder of the native warp (decision rust-gone-in-1-5): the checkout's lib/
# with the compilers warp.wasm, runtime.wasm, warp-self.wasm, modules/ and libwarp_wasmtime in it, and in that folder
# core/native/warp (`warp build native core/native/warp.warp`, which finds libwarp_wasmtime beside itself by rpath),
# since the native warp's lib/ is its own folder (core/native/driver.warp library_folder; a package links bin/warp to
# it). The gate makes the compilers' machine code for this platform, and the driver `warp build` puts programs in
# (data/native/driver), once: the folder's warp prints hello, builds a hello, and that runs. Run from the checkout's root.
set -euo pipefail
dist=$1
EXPECTED=Hello
in_dist() { (cd "$dist" && unset WARP_LIB && sh -c "$1"); }

ls lib/*warp_wasmtime.* >/dev/null || { echo "native_dist: no libwarp_wasmtime in lib/ (crates/wasmtime-capi)" >&2; exit 1; }
cp -RL lib "$dist"
rm -rf "$dist/data" "$dist"/*.cwasm "$dist"/*.inputs  # this machine's builds and machine code
cp core/native/warp "$dist/warp"

cp samples/hello/hello.warp "$dist/hello.warp"
in_dist './warp "$PWD/hello.warp"' | grep -q "$EXPECTED"
in_dist 'rm -f hello hello.*wasm && ./warp build "$PWD/hello.warp" >/dev/null && ./hello' | grep -q "$EXPECTED"
[ -x "$dist/data/native/driver" ] || { echo "native_dist: warp build put no driver in $dist/data/native (card native-install-paths)" >&2; exit 1; }
rm -f "$dist"/hello "$dist"/hello.*
echo "native_dist: $dist ready, its warp and a program it built print $EXPECTED"
