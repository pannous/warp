#!/usr/bin/env bash
# scripts/wasmtime-c-api.sh [<folder>…]: warp's native libraries built for this machine into /opt/cargo (a no-op when
# current), and copied into each folder, default the one beside the warp binary on PATH: warp's wasmtime C API
# (crates/wasmtime-capi, card wasmtime-bundled) and libwarp_sound (crates/warp-sound, miniaudio, decision sound-plays).
# after-push.sh installs them beside warp and into the main checkout's lib/, rebuild.sh into a tree's lib/, the library
# folder a native build links from and its executables find them in (rpath); the release workflow runs it for lib/
set -eu -o pipefail
checkout=$(cd "$(dirname "$0")/.." && pwd)
[ $# -gt 0 ] || set -- "$(dirname "$(readlink -f "$(command -v warp)")")"
MINIAUDIO_VERSION=0.11.21
MINIAUDIO_SHA256=6b2029714f8634c4d7c70cc042f45074e0565766113fc064f20cd27c986be9c9
export CARGO_TARGET_DIR=${WARP_WASMTIME_TARGET:-/opt/cargo/warp-wasmtime}
sound_build=${WARP_SOUND_TARGET:-/opt/cargo/warp-sound}
(cd "$checkout/crates/wasmtime-capi" && nice -n 19 cargo build --release --locked -q)
case $(uname -s) in
  Darwin) suffix=dylib; prefix=lib; system_libraries="" ;;
  Linux) suffix=so; prefix=lib; system_libraries="-ldl -lpthread -lm" ;;
  *) suffix=dll; prefix=""; system_libraries="" ;;
esac
wasmtime_library=${prefix}warp_wasmtime.$suffix
sound_library=${prefix}warp_sound.$suffix

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
done
