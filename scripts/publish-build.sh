#!/usr/bin/env bash
# scripts/publish-build.sh [<tree>]: share the compilers a tree built (its lib/warp.wasm.inputs hash) with the other
# machines, which take them in scripts/fetch-build.sh (rebuild.sh does on a store miss): the store entry
# <store>/<hash>/ (warp.wasm, warp-self.wasm, runtime.wasm, modules/*.wasm, their .inputs; no machine code, which
# is per host) as the asset build-<hash>.tar.gz of the pre-release `builds` of pannous/warp, with a SHA256SUMS manifest.
# Run after a landing (handbook "Done"). Write-once: an asset there is kept. Keeps the newest KEEP assets.
set -euo pipefail
REPO=pannous/warp TAG=builds KEEP=${WARP_BUILDS_KEEP:-40}
source "$(dirname "$0")/lib/platform.sh"
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd)
hash=$(cut -d' ' -f1 "$tree/lib/warp.wasm.inputs" 2>/dev/null) || { echo "publish-build: $tree has no built lib/warp.wasm (scripts/rebuild.sh pure)" >&2; exit 1; }
entry=$warp_store/$hash
[ -f "$entry/warp.wasm" ] || { echo "publish-build: no store entry $entry/warp.wasm" >&2; exit 1; }
asset=build-$hash.tar.gz
gh release view "$TAG" -R "$REPO" >/dev/null 2>&1 ||
	gh release create "$TAG" -R "$REPO" --prerelease --title "compiler builds by inputs hash" \
		--notes "Built compilers keyed by scripts/compiler-inputs.sh hash; scripts/fetch-build.sh installs one into a store." >/dev/null
if gh release view "$TAG" -R "$REPO" --json assets --jq '.assets[].name' | grep -qx "$asset"; then
	echo "publish-build: $asset is there already"; exit 0
fi
scratch=$(mktemp -d) && trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/$hash"
(cd "$entry" && for f in warp.wasm warp-self.wasm runtime.wasm modules/*.wasm; do
	[ -f "$f" ] || continue
	mkdir -p "$scratch/$hash/$(dirname "$f")"; cp "$f" "$scratch/$hash/$f"; [ -f "$f.inputs" ] && cp "$f.inputs" "$scratch/$hash/$f.inputs"
done)
(cd "$scratch/$hash" && find . -type f ! -name SHA256SUMS | LC_ALL=C sort | while read -r f; do sha256_of "$f"; echo "  $f"; done | paste -d '' - - > SHA256SUMS)
tar -C "$scratch" -czf "$scratch/$asset" "$hash"
gh release upload "$TAG" -R "$REPO" "$scratch/$asset" >/dev/null
echo "publish-build: $asset ($(ls "$scratch/$hash" | tr '\n' ' '))"
# the oldest beyond KEEP go
gh release view "$TAG" -R "$REPO" --json assets --jq '.assets | sort_by(.createdAt) | reverse | .['"$KEEP"':][].name' |
	while read -r old; do gh release delete-asset "$TAG" "$old" -R "$REPO" -y >/dev/null; done
