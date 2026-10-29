#!/usr/bin/env bash
# scripts/release-bootstrap.sh <target> <folder>: the newest published release's native warp for <target> (its tarball,
# packaging/native_dist.sh) unpacked into <folder>, with its libwarp_wasmtime and lib/ beside it: the warp the Release
# workflow builds the next compilers and native warp with (warp from warp, no Rust harness).
set -euo pipefail
target=${1:?usage: scripts/release-bootstrap.sh <target> <folder>} folder=${2:?usage: scripts/release-bootstrap.sh <target> <folder>}
tag=$(gh release list --repo pannous/warp --exclude-drafts --exclude-pre-releases --limit 1 --json tagName -q '.[0].tagName')
archive=warp-$tag-$target.tar.gz
mkdir -p "$folder"
gh release download "$tag" --repo pannous/warp --pattern "$archive" --dir "$folder" --clobber
tar -xzf "$folder/$archive" -C "$folder" && rm "$folder/$archive"
echo "bootstrap: the native warp of $tag for $target in $folder: $("$folder/warp" version 2>&1 | head -1)"
