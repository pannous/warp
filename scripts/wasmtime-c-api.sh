#!/usr/bin/env bash
# scripts/wasmtime-c-api.sh [<folder>…]: warp's native libraries built for this machine into /opt/cargo (a no-op when
# current), and copied into each folder, default the one beside the warp binary on PATH: warp's wasmtime C API
# (crates/wasmtime-capi, card wasmtime-bundled) libwarp_sound (crates/warp-sound, miniaudio, decision sound-plays) and libwarp_gpu (crates/warp-gpu, wgpu-native,
# card gpu-functions) and libwarp_window (crates/warp-window, SDL3, card native-window).
# after-push.sh installs them beside warp and into the main checkout's lib/, rebuild.sh into a tree's lib/, the library
# folder a native build links from and its executables find them in (rpath); the release workflow runs it for lib/
set -eu -o pipefail
checkout=$(cd "$(dirname "$0")/.." && pwd)
[ $# -gt 0 ] || set -- "$(dirname "$(readlink -f "$(command -v warp)")")"
MINIAUDIO_VERSION=0.11.21
MINIAUDIO_SHA256=6b2029714f8634c4d7c70cc042f45074e0565766113fc064f20cd27c986be9c9
export CARGO_TARGET_DIR=${WARP_WASMTIME_TARGET:-/opt/cargo/warp-wasmtime}
sound_build=${WARP_SOUND_TARGET:-/opt/cargo/warp-sound}
WGPU_VERSION=29.0.1.1
gpu_build=${WARP_GPU_TARGET:-/opt/cargo/warp-gpu}
window_build=${WARP_WINDOW_TARGET:-/opt/cargo/warp-window}
(cd "$checkout/crates/wasmtime-capi" && nice -n 19 cargo build --release --locked -q)
case $(uname -s) in
  Darwin) suffix=dylib; prefix=lib; system_libraries=""
    gpu_libraries="-framework Metal -framework QuartzCore -framework Foundation -framework CoreGraphics -framework IOKit -framework IOSurface -lobjc" ;;
  Linux) suffix=so; prefix=lib; system_libraries="-ldl -lpthread -lm"; gpu_libraries="$system_libraries" ;;
  *) suffix=dll; prefix=""; system_libraries=""; gpu_libraries="" ;;
esac
wasmtime_library=${prefix}warp_wasmtime.$suffix
sound_library=${prefix}warp_sound.$suffix
gpu_library=${prefix}warp_gpu.$suffix
window_library=${prefix}warp_window.$suffix

# miniaudio's one header, fetched once at its pinned version and checked
sound_library_built() {
  mkdir -p "$sound_build"
  header=$sound_build/miniaudio-$MINIAUDIO_VERSION/miniaudio.h
  if [ ! -f "$header" ]; then
    mkdir -p "$(dirname "$header")"
    curl -sSfL -o "$header.new" "https://raw.githubusercontent.com/mackron/miniaudio/$MINIAUDIO_VERSION/miniaudio.h"
    echo "$MINIAUDIO_SHA256  $header.new" | shasum -a 256 -c - >/dev/null
    mv "$header.new" "$header"
  fi
  source=$checkout/crates/warp-sound/warp_sound.c
  built=$sound_build/$sound_library
  [ "$built" -nt "$source" ] && [ "$built" -nt "$header" ] && return
  # shellcheck disable=SC2086 # the system libraries are words
  nice -n 19 cc -O2 -shared -fPIC -I "$(dirname "$header")" -o "$built.new" "$source" $system_libraries
  mv -f "$built.new" "$built"
}
sound_library_built

# wgpu-native's release for this machine, fetched once at its pinned version and checked, linked in whole
wgpu_release_sha256() {
  case $1 in
    macos-aarch64) echo a5797a37b1adf720bcd5dcffb291edbbd5b7b14be0a3874c28e6393a655a7a3e ;;
    macos-x86_64) echo 8e2f7378548ddd0e2cf21e7d864dda46e953f0af724855a33778b85ead206d41 ;;
    linux-aarch64) echo 015fcdf1dbae82e614a783cc38017e5399ae0927a889fe9b69c9b664bc61b47a ;;
    linux-x86_64) echo 95a4d90c071005a98d03eab348beaa6b07e16eb00d1dcdb9f8348f75eb97ec5a ;;
  esac
}
gpu_library_built() {
  platform=$(uname -s | sed 's/Darwin/macos/;s/Linux/linux/')-$(uname -m | sed 's/arm64/aarch64/')
  sha256=$(wgpu_release_sha256 "$platform")
  [ -n "$sha256" ] || return 0
  release=$gpu_build/wgpu-$WGPU_VERSION
  archive=$release/lib/libwgpu_native.a
  if [ ! -f "$archive" ]; then
    mkdir -p "$release"
    curl -sSfL -o "$release.zip" "https://github.com/gfx-rs/wgpu-native/releases/download/v$WGPU_VERSION/wgpu-$platform-release.zip"
    echo "$sha256  $release.zip" | shasum -a 256 -c - >/dev/null
    (cd "$release" && unzip -oq "$release.zip") && rm "$release.zip"
  fi
  source=$checkout/crates/warp-gpu/warp_gpu.c
  built=$gpu_build/$gpu_library
  [ "$built" -nt "$source" ] && [ "$built" -nt "$archive" ] && return
  # shellcheck disable=SC2086 # the system libraries are words
  nice -n 19 cc -O2 -shared -fPIC -I "$release/include" -o "$built.new" "$source" "$archive" $gpu_libraries
  mv -f "$built.new" "$built"
}
gpu_library_built

# SDL3 as the system installs it (brew install sdl3, apt install libsdl3-dev), linked as a library of its own
window_library_built() {
  flags=$(pkg-config --cflags --libs sdl3 2>/dev/null) || return 0
  mkdir -p "$window_build"
  source=$checkout/crates/warp-window/warp_window.c
  built=$window_build/$window_library
  [ "$built" -nt "$source" ] && return
  # shellcheck disable=SC2086 # the flags are words
  nice -n 19 cc -O2 -shared -fPIC -o "$built.new" "$source" $flags
  mv -f "$built.new" "$built"
}
window_library_built

# each library moved into place whole: a run loading it meanwhile meets the old one or the new one
installed() {
  built=$1 folder=$2 name=$(basename "$1")
  cmp -s "$built" "$folder/$name" && return
  cp "$built" "$folder/$name.new" && mv -f "$folder/$name.new" "$folder/$name"
  echo "wasmtime-c-api: $folder/$name"
}
for folder in "$@"; do
  installed "$CARGO_TARGET_DIR/release/$wasmtime_library" "$folder"
  installed "$sound_build/$sound_library" "$folder"
  # (a platform wgpu-native ships no release for gets none: its programs paint on the CPU)
  if [ -f "$gpu_build/$gpu_library" ]; then installed "$gpu_build/$gpu_library" "$folder"; fi
  # (a machine without SDL3 gets none: its programs run their frames once, windowless)
  if [ -f "$window_build/$window_library" ]; then installed "$window_build/$window_library" "$folder"; fi
done
