#!/usr/bin/env bash
# scripts/fetch-build.sh <hash> [<store>] [<file>]: the compilers of an inputs hash another machine built and published
# (scripts/publish-build.sh) into <store>/<hash>/ (default platform.sh warp_store), and the self-tested mark they
# carry into <store>/self-tested/. Exit 0 when the store has <file> (default warp.wasm) of the hash afterwards, 1 when none is published or it fails its checks. Never overwrites a file of the store: a
# missing entry is renamed in whole, a partial one only gets the files it lacks. Each file is checked against the
# SHA256SUMS manifest and each .wasm for the wasm header before anything is installed.
set -uo pipefail
REPO=pannous/warp TAG=builds
source "$(dirname "$0")/lib/platform.sh"
hash=${1:?usage: fetch-build.sh <hash> [<store>] [<file>]} store=${2:-$warp_store} wanted=${3:-warp.wasm}
[[ $hash =~ ^[0-9a-f]{16}$ ]] || { echo "fetch-build: $hash is no inputs hash" >&2; exit 2; }
[ -f "$store/$hash/$wanted" ] && exit 0
command -v gh >/dev/null || exit 1
scratch=$(mktemp -d "${TMPDIR:-/tmp}/fetch-build.XXXXXX") && trap 'rm -rf "$scratch"' EXIT
gh release download "$TAG" -R "$REPO" -p "build-$hash.tar.gz" -D "$scratch" >/dev/null 2>&1 || exit 1
tar -C "$scratch" -xzf "$scratch/build-$hash.tar.gz" "$hash" || exit 1
got=$scratch/$hash
[ -f "$got/SHA256SUMS" ] && [ -f "$got/warp.wasm" ] || { echo "fetch-build: build-$hash has no manifest or warp.wasm" >&2; exit 1; }
listed=$(cd "$got" && find . -type f ! -name SHA256SUMS | LC_ALL=C sort)
[ "$listed" = "$(sed 's/^[0-9a-f]*  //' "$got/SHA256SUMS")" ] || { echo "fetch-build: build-$hash: files and manifest differ" >&2; exit 1; }
while read -r sum file; do
	[ "$(cd "$got" && sha256_of "$file")" = "$sum" ] || { echo "fetch-build: build-$hash: $file fails its checksum" >&2; exit 1; }
	case $file in *.wasm) [ "$(head -c 4 "$got/$file" | od -An -tx1 | tr -d ' \n')" = 0061736d ] || { echo "fetch-build: $file is no wasm" >&2; exit 1; } ;; esac
done < "$got/SHA256SUMS"
rm "$got/SHA256SUMS"
mkdir -p "$store"
# the self test passed for these baked inputs on the publishing machine (rebuild.sh self_tested): kept unless here
if [ -d "$got/self-tested" ]; then
	mkdir -p "$store/self-tested"
	for mark in "$got"/self-tested/*; do [ -e "$store/self-tested/${mark##*/}" ] || cp "$mark" "$store/self-tested/${mark##*/}"; done
	rm -rf "$got/self-tested"
fi
if mv -T "$got" "$store/$hash" 2>/dev/null || { [ ! -e "$store/$hash" ] && mv "$got" "$store/$hash"; }; then
	:
else
	(cd "$got" && find . -type f) | while read -r file; do
		mkdir -p "$(dirname "$store/$hash/$file")"
		[ -e "$store/$hash/$file" ] || { cp "$got/$file" "$store/$hash/$file.fetched-$$" && mv -n "$store/$hash/$file.fetched-$$" "$store/$hash/$file"; rm -f "$store/$hash/$file.fetched-$$"; }
	done
fi
echo "fetch-build: build-$hash into $store/$hash"
[ -f "$store/$hash/$wanted" ]
