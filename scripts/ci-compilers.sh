#!/usr/bin/env bash
# scripts/ci-compilers.sh <tree> <folder>: <tree>'s own compilers in <tree>/lib (warp.wasm, runtime.wasm, modules/), built
# on a linux x86_64 runner, for the Release workflow and the Playground (pages.yml: the page must run the compiler of
# the lib/ it serves, not the last release's, card safari-painting-checks). Installs wabt and binaryen on the PATH,
# builds a native warp into <folder> (scripts/release-bootstrap.sh) and has the newest seed rebuild the compilers.
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
gh release download "$seed" --repo pannous/warp --pattern warp-seed.wasm --dir "$folder" --clobber
cd "$tree" && WARP_LIB=$tree/lib "$folder/warp" rebuild pure
