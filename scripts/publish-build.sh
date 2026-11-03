#!/usr/bin/env bash
# scripts/publish-build.sh [<tree>]: share the compilers a tree built (its lib/warp.wasm.inputs hash) with the other
# machines, which take them in scripts/fetch-build.sh (rebuild.sh does on a store miss): the store entry
# <store>/<hash>/ (warp.wasm, warp-self.wasm, runtime.wasm, modules/*.wasm, their .inputs; no machine code, which
# is per host) as the asset build-<hash>.tar.gz of the pre-release `builds` of pannous/warp, with a SHA256SUMS manifest.
# With a passed self test (<store>/self-tested mark) also build-<hash>-self.tar.gz: warp-self.wasm and the marks.
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
present=$(gh release view "$TAG" -R "$REPO" --json assets --jq '.assets[].name')
scratch=$(mktemp -d) && trap 'rm -rf "$scratch"' EXIT
# pack <asset> <folder>: <folder> (named <hash>) with its SHA256SUMS manifest as <asset>, uploaded; one there is kept
pack() {
	if grep -qx "$1" <<<"$present"; then echo "publish-build: $1 is there already"; return; fi
	(cd "$2" && find . -type f ! -name SHA256SUMS | LC_ALL=C sort | while read -r f; do sha256_of "$f"; echo "  $f"; done | paste -d '' - - > SHA256SUMS)
	tar -C "$(dirname "$2")" -czf "$scratch/$1" "$hash"
	gh release upload "$TAG" -R "$REPO" "$scratch/$1" >/dev/null
	echo "publish-build: $1 ($(ls "$2" | tr '\n' ' '))"
}
# the self-tested marks of this hash (rebuild.sh self_passed: <store>/self-tested/<baked hash>, its line naming the hash)
marks=()
for mark in "$warp_store"/self-tested/*; do [ -f "$mark" ] && [ "$(cut -d' ' -f1 "$mark")" = "$hash" ] && marks+=("$mark"); done
# and the mark of its code (the baked hash its build recorded) when this hash's own self test passed (self-check.sh
# stage two, <entry>/stage-two.passed) though the mark names the hash first self-tested: a lib/-only change, its
# warp-self.wasm promoted by that mark (rebuild.sh hit-baked; card publish-self-mark)
baked=$(awk '$1=="baked"{print $2; exit}' "$entry/warp.record/inputs.txt" 2>/dev/null)
mark=$warp_store/self-tested/$baked
if [ -n "$baked" ] && [ -f "$mark" ] && [ "$(cut -d' ' -f1 "$entry/stage-two.passed" 2>/dev/null)" = "$hash" ] &&
	[[ " ${marks[*]-} " != *" $mark "* ]]; then marks+=("$mark"); fi
copy_into() { local folder=$1 f; shift; for f in "$@"; do [ -f "$entry/$f" ] || continue; mkdir -p "$folder/$(dirname "$f")"; cp "$entry/$f" "$folder/$f"; [ -f "$entry/$f.inputs" ] && cp "$entry/$f.inputs" "$folder/$f.inputs"; done; }
mkdir -p "$scratch/main/$hash"
(cd "$entry" && copy_into "$scratch/main/$hash" warp.wasm warp-self.wasm runtime.wasm modules/*.wasm modules/words.tsv)
((${#marks[@]})) && { mkdir -p "$scratch/main/$hash/self-tested"; cp "${marks[@]}" "$scratch/main/$hash/self-tested/"; }
pack "$asset" "$scratch/main/$hash"
# the self build and its marks apart (build-<hash>-self.tar.gz), only for a hash whose self test passed here: a main
# asset published before the self build stays as it is (write-once), this one adds what it lacked
if ((${#marks[@]})) && [ -f "$entry/warp-self.wasm" ]; then
	mkdir -p "$scratch/self/$hash/self-tested"
	copy_into "$scratch/self/$hash" warp-self.wasm
	cp "${marks[@]}" "$scratch/self/$hash/self-tested/"
	pack "build-$hash-self.tar.gz" "$scratch/self/$hash"
fi
# the oldest beyond KEEP go
gh release view "$TAG" -R "$REPO" --json assets --jq '.assets | sort_by(.createdAt) | reverse | .['"$KEEP"':][].name' |
	while read -r old; do gh release delete-asset "$TAG" "$old" -R "$REPO" -y >/dev/null; done
