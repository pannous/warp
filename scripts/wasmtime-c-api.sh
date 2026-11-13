#!/usr/bin/env bash
# scripts/wasmtime-c-api.sh [--from <tree>] [<folder>…]: warp's native libraries built for this machine into /opt/cargo (a no-op when
# current), and copied into each folder, default the one beside the warp binary on PATH: warp's wasmtime C API
# (crates/wasmtime-capi, card wasmtime-bundled) and wgpu-native's release (libwgpu_native, card gpu-functions). The
# window, the sound, the web view and the GPU are warp's own (lib/host/native_host.warp, over SDL3, WebKit, wgpu-native and
# the system's libraries by FFI).
# after-push.sh installs them beside warp and into the main checkout's lib/, rebuild.sh into a tree's lib/, the library
# folder a native build links from and its executables find them in (rpath); the release workflow runs it for lib/
set -eu -o pipefail
# the tree whose native warp links it (rebuild.sh, after-push.sh) builds it with its own copy of this script: the
# checkout of the script running may lag that tree, its sources (card after-push-native-warp-1113) and how they are
# built alike (an older copy applied the tree's patches to the wrong folder, card after-push-wasmtime-c-api-1113)
if [ "${1-}" = --from ]; then
  tree_script=$(cd "$2" && pwd)/scripts/wasmtime-c-api.sh
  shift 2
  [ "$tree_script" -ef "$0" ] || exec "$tree_script" "$@"
fi
checkout=$(cd "$(dirname "$0")/.." && pwd)
[ $# -gt 0 ] || set -- "$(dirname "$(readlink -f "$(command -v warp)")")"
export CARGO_TARGET_DIR=${WARP_WASMTIME_TARGET:-/opt/cargo/warp-wasmtime}
WGPU_VERSION=29.0.1.1
gpu_build=${WARP_GPU_TARGET:-/opt/cargo/warp-gpu}
WASMTIME_VERSION=50.0.0
# the crates warp patches (crates/wasmtime-capi/patches/, its Cargo.toml [patch.crates-io]), each with its sha256
PATCHED_CRATES="wasmtime:2d93539aae382faee7b19650fd966f1f8185599b9cc4adc0a5a8b8f768c11ba9
wasmtime-environ:34fd44c205ef29cc3c254253dceb916be586887602e0e380361ed022776cfc10
wasmtime-internal-cranelift:1f496bf29d3f2a1d060e0a497a20bfba2a36befe80bdf3e640ba8d3dcab7df35"

# fetched <file> <url> <sha256>: the file downloaded once and checked
fetched() {
  [ -f "$1" ] && return
  mkdir -p "$(dirname "$1")"
  curl -sSfL -o "$1.new" "$2"
  echo "$3  $1.new" | shasum -a 256 -c - >/dev/null
  mv "$1.new" "$1"
}

# wasmtime with warp's patches (crates/wasmtime-capi/patches/, paths from the folder holding the crates): each
# patched crate from crates.io, unpacked into crates/wasmtime-capi/patched/<crate> and patched anew whenever the
# patches change
patched_wasmtime() {
  local crate=$checkout/crates/wasmtime-capi stamp name sha256 archive
  local patched=$crate/patched
  stamp=$(cat "$crate"/patches/*.patch | shasum -a 256 | cut -d' ' -f1)
  [ "$(cat "$patched/.patches" 2>/dev/null)" = "$stamp" ] && return
  rm -rf "$patched" && mkdir -p "$patched"
  for entry in $PATCHED_CRATES; do
    name=${entry%%:*} sha256=${entry#*:}
    archive=$CARGO_TARGET_DIR/$name-$WASMTIME_VERSION.crate
    fetched "$archive" "https://static.crates.io/crates/$name/$name-$WASMTIME_VERSION.crate" "$sha256"
    tar -xzf "$archive" -C "$patched"
    mv "$patched/$name-$WASMTIME_VERSION" "$patched/$name"
  done
  for patch in "$crate"/patches/*.patch; do patch -s -p1 -d "$patched" <"$patch"; done
  echo "$stamp" >"$patched/.patches"
}
# the C API built once per set of inputs: a fresh worktree unpacks wasmtime at a new path, which cargo takes for a
# new crate and rebuilds all of wasmtime from scratch (a rustc in every landing's rebuild)
wasmtime_inputs=$( (echo "$WASMTIME_VERSION"; cd "$checkout/crates/wasmtime-capi" && cat Cargo.toml Cargo.lock build.rs patches/*.patch $(find src -type f | sort)) | shasum -a 256 | cut -d' ' -f1)
wasmtime_stamp=$CARGO_TARGET_DIR/release/warp_wasmtime.inputs
if [ "$(cat "$wasmtime_stamp" 2>/dev/null)" != "$wasmtime_inputs" ] || ! ls "$CARGO_TARGET_DIR"/release/*warp_wasmtime.* >/dev/null 2>&1; then
  patched_wasmtime
  (cd "$checkout/crates/wasmtime-capi" && nice -n 19 cargo build --release --locked -q)
  echo "$wasmtime_inputs" >"$wasmtime_stamp"
fi
case $(uname -s) in
  Darwin) suffix=dylib; prefix=lib ;;
  Linux) suffix=so; prefix=lib ;;
  *) suffix=dll; prefix="" ;;
esac
wasmtime_library=${prefix}warp_wasmtime.$suffix
gpu_library=${prefix}wgpu_native.$suffix

# wgpu-native's release for this machine, fetched once at its pinned version and checked: its shared library is what
# lib/host/native_host/gpu.warp calls
wgpu_release_sha256() {
  case $1 in
    macos-aarch64) echo a5797a37b1adf720bcd5dcffb291edbbd5b7b14be0a3874c28e6393a655a7a3e ;;
    macos-x86_64) echo 8e2f7378548ddd0e2cf21e7d864dda46e953f0af724855a33778b85ead206d41 ;;
    linux-aarch64) echo 015fcdf1dbae82e614a783cc38017e5399ae0927a889fe9b69c9b664bc61b47a ;;
    linux-x86_64) echo 95a4d90c071005a98d03eab348beaa6b07e16eb00d1dcdb9f8348f75eb97ec5a ;;
  esac
}
gpu_release() {
  platform=$(uname -s | sed 's/Darwin/macos/;s/Linux/linux/')-$(uname -m | sed 's/arm64/aarch64/')
  sha256=$(wgpu_release_sha256 "$platform")
  [ -n "$sha256" ] || return 0
  release=$gpu_build/wgpu-$WGPU_VERSION
  if [ ! -f "$release/lib/$gpu_library" ]; then
    fetched "$release.zip" "https://github.com/gfx-rs/wgpu-native/releases/download/v$WGPU_VERSION/wgpu-$platform-release.zip" "$sha256"
    mkdir -p "$release" && (cd "$release" && unzip -oq "$release.zip") && rm "$release.zip"
  fi
  echo "$release/lib/$gpu_library"
}
gpu_built=$(gpu_release)

# each library moved into place whole: a run loading it meanwhile meets the old one or the new one
installed() {
  built=$1 folder=$2 name=$(basename "$1")
  cmp -s "$built" "$folder/$name" && return
  cp "$built" "$folder/$name.new" && mv -f "$folder/$name.new" "$folder/$name"
  echo "wasmtime-c-api: $folder/$name"
}
for folder in "$@"; do
  installed "$CARGO_TARGET_DIR/release/$wasmtime_library" "$folder"
  # (a platform wgpu-native ships no release for gets none: its programs paint on the CPU)
  if [ -n "$gpu_built" ]; then installed "$gpu_built" "$folder"; fi
done
