#!/usr/bin/env bash
# scripts/release-bootstrap.sh <target> <folder> [<tree>]: the newest published release's native warp for <target> (its
# tarball, packaging/native_dist.sh) unpacked into <folder>, with its libwarp_wasmtime and lib/ beside it: the warp the
# Release workflow builds the next compilers and native warp with (warp from warp, no Rust harness).
# With <tree>, that warp is brought up to the tree's host words first: an older native warp lacks words the tree's
# compilers import (v1.5.2 has no real_path), so it builds the tree's core/native/warp.warp with its own compilers and
# the tree's lib/native C sources, and the result replaces it. Needs wasm2c on the PATH.
set -euo pipefail
usage="usage: scripts/release-bootstrap.sh <target> <folder> [<tree>]"
target=${1:?$usage} folder=${2:?$usage} tree=${3:-}
tag=$(gh release list --repo pannous/warp --exclude-drafts --exclude-pre-releases --limit 1 --json tagName -q '.[0].tagName')
archive=warp-$tag-$target.tar.gz
mkdir -p "$folder"
gh release download "$tag" --repo pannous/warp --pattern "$archive" --dir "$folder" --clobber
tar -xzf "$folder/$archive" -C "$folder" && rm "$folder/$archive"
echo "bootstrap: the native warp of $tag for $target in $folder: $("$folder/warp" version 2>&1 | head -1)"
[ -n "$tree" ] || exit 0
folder=$(cd "$folder" && pwd)
cp "$tree"/lib/native/*.[ch] "$folder/native/"
(cd "$tree/core/native" && WARP_LIB=$folder "$folder/warp" build native warp.warp)
mv "$tree/core/native/warp" "$folder/warp"
echo "bootstrap: rebuilt from $tree/core/native/warp.warp: $("$folder/warp" version 2>&1 | head -1)"
