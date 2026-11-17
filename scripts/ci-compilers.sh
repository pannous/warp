#!/usr/bin/env bash
# scripts/ci-compilers.sh <tree> <folder>: <tree>'s own compilers in <tree>/lib (warp.wasm, runtime.wasm, modules/), built
# on a linux x86_64 runner, for the Release workflow and the Playground (pages.yml: the page must run the compiler of
# the lib/ it serves, not the last release's, card safari-painting-checks). Installs wabt and binaryen on the PATH,
# builds a runner warp into <folder> (scripts/release-bootstrap.sh), has the newest seed rebuild the compilers, and builds
# the tree's own native warp with them.
set -euo pipefail
WABT=${WABT:-1.0.42}     # wasm2c, which `warp build native` runs
BINARYEN=${BINARYEN:-133}    # wasm-opt, which `warp build` runs
TARGET=x86_64-unknown-linux-gnu
usage="usage: scripts/ci-compilers.sh <tree> <folder>"
tree=$(cd "${1:?$usage}" && pwd)
mkdir -p "${2:?$usage}" && folder=$(cd "$2" && pwd)
tools=${RUNNER_TEMP:?a GitHub runner}

gh release download "version_$BINARYEN" --repo WebAssembly/binaryen --pattern "binaryen-version_$BINARYEN-x86_64-linux.tar.gz" --dir "$tools" --clobber
tar -xzf "$tools/binaryen-version_$BINARYEN-x86_64-linux.tar.gz" -C "$tools"
gh release download "$WABT" --repo WebAssembly/wabt --pattern "wabt-$WABT-linux-x64.tar.gz" --dir "$tools" --clobber
tar -xzf "$tools/wabt-$WABT-linux-x64.tar.gz" -C "$tools"
export PATH="$tools/binaryen-version_$BINARYEN/bin:$tools/wabt-$WABT/bin:$PATH"
echo "$tools/binaryen-version_$BINARYEN/bin" >>"${GITHUB_PATH:-/dev/null}"
echo "$tools/wabt-$WABT/bin" >>"${GITHUB_PATH:-/dev/null}"

"$(dirname "$0")/release-bootstrap.sh" "$TARGET" "$folder" "$tree"
seed=$(gh release list --repo pannous/warp --limit 100 --json tagName -q '[.[] | select(.tagName | startswith("seed-"))][0].tagName')
gh release download "$seed" --repo pannous/warp --pattern 'warp-seed*.wasm' --dir "$folder" --clobber
# the seed compares texts by NFC and the runner has no installed lib/: it links the runtime and unicode module it built
# of its own source, published beside it (scripts/refresh-seed.sh), until the rebuild writes the tree's own
mkdir -p "$tree/lib/modules"
cp "$folder/warp-seed-runtime.wasm" "$tree/lib/runtime.wasm"
cp "$folder/warp-seed-unicode.wasm" "$tree/lib/modules/unicode.wasm"
cd "$tree" && "$folder/warp" --lib "$tree/lib" rebuild pure
# the native warp of this tree itself, built with the compilers just made: the runner above came from the source the
# previous release could emit (scripts/bootstrap-native.ref), and lacks the host words this tree imports
# it links the tree's own wasmtime C API from <tree>/lib, built there first by the release workflow (its setters the
# runner's lacks: warp_config_wasm_stack_set after v1.11.0), else the runner's ("cannot find -lwarp_wasmtime"); and
# finds it beside itself, where the runner's lay
ls "$tree/lib"/libwarp_wasmtime.* >/dev/null 2>&1 || cp "$folder"/libwarp_wasmtime.* "$tree/lib/"
(cd "$tree/core/native" && "$folder/warp" --lib "$tree/lib" build native warp.warp) && mv "$tree/core/native/warp" "$folder/warp" &&
  cp "$tree/lib"/libwarp_wasmtime.* "$folder/"
