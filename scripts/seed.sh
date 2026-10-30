#!/usr/bin/env bash
# scripts/seed.sh [<tree>]: the path of the seed that builds the tree's warp.wasm, fetched once into the store. The
# tree's seed.lock names it (its release and SHA-256; refresh-seed.sh writes it with each new seed); a commit older than
# seed.lock gets the newest seed published before it. Every seed stays a GitHub pre-release seed-<date>-<sha8>, so any
# commit's compiler can be built again (user 2026-10-30: "we must be able to reconstruct them")
set -uo pipefail
REPO=pannous/warp
if [ "$(uname)" = Darwin ]; then STORE=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}/seeds; else STORE=${WARP_COMPILER_STORE:-$HOME/.cache/warp/compilers}/seeds; fi
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
lock=$tree/seed.lock
fail() { echo "seed.sh: $*" >&2; exit 1; }

release= sha=
if [ -f "$lock" ]; then
	read -r release sha <"$lock"
else
	# the commit's time in UTC, as GitHub dates its releases
	seconds=$(git -C "$tree" log -1 --format=%ct HEAD 2>/dev/null) || fail "$tree is no git tree"
	when=$(date -u -d "@$seconds" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -r "$seconds" +%Y-%m-%dT%H:%M:%SZ)
	release=$(gh release list -R "$REPO" -L 1000 --json tagName,publishedAt \
		--jq "[.[] | select(.tagName | startswith(\"seed-\")) | select(.publishedAt < \"$when\")] | sort_by(.publishedAt) | last | .tagName // empty")
	[ -n "$release" ] || fail "no seed published before $when (a commit before seed-2026-10-26: build one with the Rust compiler, branch rust-bootstrap)"
	echo "seed.sh: $tree has no seed.lock, the newest seed before $when: $release" >&2
fi
file=$STORE/$release.wasm
if [ ! -f "$file" ]; then
	mkdir -p "$STORE" || fail "no store at $STORE"
	gh release download "$release" -R "$REPO" -p warp-seed.wasm -O "$file.part-$$" --clobber >/dev/null 2>&1 ||
		{ rm -f "$file.part-$$"; fail "release $release could not be fetched"; }
	mv "$file.part-$$" "$file"
fi
if [ -n "$sha" ]; then
	got=$({ sha256sum "$file" 2>/dev/null || shasum -a 256 "$file"; } | cut -c1-64)
	[ "$got" = "$sha" ] || fail "$file is $got, seed.lock says $sha"
fi
echo "$file"
