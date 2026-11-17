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
# The runner is built from the source the previous release's compiler can still emit: the tree's own native source may
# use a primitive feature newer than that release (fixed arrays, 3d4864c0a9: "primitive mode cannot emit `… # …`"). The
# tree names that commit in scripts/bootstrap-native.ref (a tree without one builds from itself); it moves forward with
# each release, and the final native warp is built with the tree's own fresh compilers (ci-compilers.sh, release.yml).
runner_source=$tree
ref_file=$tree/scripts/bootstrap-native.ref
if [ -f "$ref_file" ]; then
	ref=$(awk '{ print $1; exit }' "$ref_file")
	git -C "$tree" cat-file -e "$ref^{commit}" 2>/dev/null || git -C "$tree" fetch -q --depth=1 origin "$ref"
	runner_source=$folder/runner-source
	rm -rf "$runner_source" && mkdir -p "$runner_source"
	git -C "$tree" archive "$ref" core/native lib/native | tar -x -C "$runner_source"
fi
cp "$runner_source"/lib/native/* "$previous/native/"
# TODO (card no-bash-exports) the published release's warp predates --lib and reads WARP_LIB: `--lib "$previous"` once
# a release takes it
(cd "$runner_source/core/native" && WARP_LIB=$previous "$previous/warp" build native warp.warp)
mv "$runner_source/core/native/warp" "$folder/warp"
cp "$previous"/libwarp_wasmtime.* "$folder/"
echo "bootstrap: $folder/warp built from $runner_source/core/native/warp.warp by it"
