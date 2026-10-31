#!/usr/bin/env bash
# scripts/release-bootstrap.sh <target> <folder> <tree>: a native warp for <target> in <folder>, with its
# libwarp_wasmtime beside it, that runs <tree>'s compilers: the warp the Release workflow builds the next compilers and
# native warp with (warp from warp, no Rust harness). The newest published release's native warp (its tarball,
# packaging/native_dist.sh, unpacked into <folder>/previous) lacks host words a newer tree imports (v1.5.2 has no
# real_path), so it builds <tree>'s core/native/warp.warp with its own compilers and <tree>'s lib/native. <folder>
# holds no lib/ of its own: a warp beside an older lib/ would take that lib/'s runtime. Needs wasm2c on the PATH.
set -euo pipefail
usage="usage: scripts/release-bootstrap.sh <target> <folder> <tree>"
target=${1:?$usage} folder=${2:?$usage} tree=${3:?$usage}
tag=$(gh release list --repo pannous/warp --exclude-drafts --exclude-pre-releases --limit 1 --json tagName -q '.[0].tagName')
archive=warp-$tag-$target.tar.gz
mkdir -p "$folder/previous"
folder=$(cd "$folder" && pwd) previous=$(cd "$folder/previous" && pwd)
gh release download "$tag" --repo pannous/warp --pattern "$archive" --dir "$previous" --clobber
tar -xzf "$previous/$archive" -C "$previous" && rm "$previous/$archive"
echo "bootstrap: the native warp of $tag for $target: $("$previous/warp" version 2>&1 | head -1)"
cp "$tree"/lib/native/* "$previous/native/"
(cd "$tree/core/native" && WARP_LIB=$previous "$previous/warp" build native warp.warp)
mv "$tree/core/native/warp" "$folder/warp"
cp "$previous"/libwarp_wasmtime.* "$folder/"
echo "bootstrap: $folder/warp built from $tree/core/native/warp.warp by it"
