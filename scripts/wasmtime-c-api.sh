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
STB_COMMIT=6e9f34d5429cf16790ec43c9bac3f1ee4ad1f760
STB_TRUETYPE_SHA256=ecd30b05e0dd4fea3a13c26810dd9e1992dc379049482c393d5a19e6b5090aab
gpu_build=${WARP_GPU_TARGET:-/opt/cargo/warp-gpu}
window_build=${WARP_WINDOW_TARGET:-/opt/cargo/warp-window}
WASMTIME_VERSION=50.0.0
WASMTIME_SHA256=2d93539aae382faee7b19650fd966f1f8185599b9cc4adc0a5a8b8f768c11ba9

# fetched <file> <url> <sha256>: the file downloaded once and checked
fetched() {
  [ -f "$1" ] && return
  mkdir -p "$(dirname "$1")"
  curl -sSfL -o "$1.new" "$2"
  echo "$3  $1.new" | shasum -a 256 -c - >/dev/null
  mv "$1.new" "$1"
}

# wasmtime with warp's patches (crates/wasmtime-capi/patches/, its Cargo.toml [patch.crates-io]): the crate from
# crates.io, unpacked into crates/wasmtime-capi/patched/wasmtime and patched anew whenever the patches change
patched_wasmtime() {
  local crate=$checkout/crates/wasmtime-capi stamp
  local patched=$crate/patched
  stamp=$(cat "$crate"/patches/*.patch | shasum -a 256 | cut -d' ' -f1)
  [ "$(cat "$patched/wasmtime/.patches" 2>/dev/null)" = "$stamp" ] && return
  fetched "$CARGO_TARGET_DIR/wasmtime-$WASMTIME_VERSION.crate" "https://static.crates.io/crates/wasmtime/wasmtime-$WASMTIME_VERSION.crate" "$WASMTIME_SHA256"
  rm -rf "$patched/wasmtime" "$patched/wasmtime-$WASMTIME_VERSION" && mkdir -p "$patched"
  tar -xzf "$CARGO_TARGET_DIR/wasmtime-$WASMTIME_VERSION.crate" -C "$patched"
  mv "$patched/wasmtime-$WASMTIME_VERSION" "$patched/wasmtime"
  for patch in "$crate"/patches/*.patch; do patch -s -p1 -d "$patched/wasmtime" <"$patch"; done
  echo "$stamp" >"$patched/wasmtime/.patches"
}
patched_wasmtime
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

# a library owning host words reads and builds warp values in place (lib/native/warp_node.h) through warp's wasmtime:
# its headers, it linked, and the value layout as the compiler has it (scripts/node_layout.warp), checked when a warp
# is at hand: the committed header differing fails the build
# (cargo's build folder of the C API, <package>-<hash>/out or, newer, <package>/<hash>/out)
wasmtime_include=$(find "$CARGO_TARGET_DIR/release/build" -path '*wasmtime-c-api-impl*/out/include' -type d -print -quit)
[ -n "$wasmtime_include" ] || { echo "wasmtime-c-api.sh: no wasmtime C API headers under $CARGO_TARGET_DIR/release/build" >&2; exit 1; }
case $(uname -s) in
  Darwin) loader_path=@loader_path ;;
  *) loader_path='$ORIGIN' ;;
esac
value_libraries="-I $checkout/lib/native -I $wasmtime_include -L $CARGO_TARGET_DIR/release -lwarp_wasmtime -Wl,-rpath,$loader_path"
value_headers="$checkout/lib/native/warp_node.h $checkout/lib/native/warp_layout.h"
if command -v warp >/dev/null; then
  layout=$(cd "$checkout" && WARP_NO_WINDOW=1 warp scripts/node_layout.warp)
  if [ "$layout" != "$(cat "$checkout/lib/native/warp_layout.h")" ]; then
    echo "wasmtime-c-api: the value layout changed: warp scripts/node_layout.warp > lib/native/warp_layout.h, and commit it" >&2
    exit 1
  fi
fi
# a library is current when newer than each of its sources
current() { local target=$1 each; shift; for each in "$@"; do [ "$target" -nt "$each" ] || return 1; done; }

# miniaudio's one header, fetched once at its pinned version and checked
sound_library_built() {
  mkdir -p "$sound_build"
  header=$sound_build/miniaudio-$MINIAUDIO_VERSION/miniaudio.h
  fetched "$header" "https://raw.githubusercontent.com/mackron/miniaudio/$MINIAUDIO_VERSION/miniaudio.h" "$MINIAUDIO_SHA256"
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
    fetched "$release.zip" "https://github.com/gfx-rs/wgpu-native/releases/download/v$WGPU_VERSION/wgpu-$platform-release.zip" "$sha256"
    mkdir -p "$release" && (cd "$release" && unzip -oq "$release.zip") && rm "$release.zip"
  fi
  source=$checkout/crates/warp-gpu/warp_gpu.c
  built=$gpu_build/$gpu_library
  # shellcheck disable=SC2086 # the headers are words
  current "$built" "$source" "$archive" $value_headers && return
  # shellcheck disable=SC2086 # the system libraries are words
  nice -n 19 cc -O2 -shared -fPIC -I "$release/include" -o "$built.new" "$source" "$archive" $gpu_libraries $value_libraries
  mv -f "$built.new" "$built"
}
gpu_library_built

# SDL3 as the system installs it (brew install sdl3, apt install libsdl3-dev), linked as a library of its own, with
# stb_truetype's one header (text_coverage), fetched once at its pinned commit and checked
window_library_built() {
  flags=$(pkg-config --cflags --libs sdl3 2>/dev/null) || return 0
  mkdir -p "$window_build"
  header=$window_build/stb-$STB_COMMIT/stb_truetype.h
  fetched "$header" "https://raw.githubusercontent.com/nothings/stb/$STB_COMMIT/stb_truetype.h" "$STB_TRUETYPE_SHA256"
  source=$checkout/crates/warp-window/warp_window.c
  built=$window_build/$window_library
  # shellcheck disable=SC2086 # the headers are words
  current "$built" "$source" "$header" $value_headers && return
  # shellcheck disable=SC2086 # the flags are words
  nice -n 19 cc -O2 -shared -fPIC -I "$(dirname "$header")" -o "$built.new" "$source" $flags $value_libraries
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
