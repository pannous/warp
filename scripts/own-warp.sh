#!/bin/bash
# Build the warp CLI and keep a private copy under scratch/warp.
# The shared cargo target dir's debug/warp is one file for every checkout: another worktree's build overwrites it.
# Usage (from repo root or anywhere): scripts/own-warp.sh
# Prints the absolute path of scratch/warp on success.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

# A worktree builds only with its build tweak (scripts/version-tweak.sh): untweaked, the shared target dir links
# another checkout's warp, whose lib/ path the binary then reads (env! CARGO_MANIFEST_DIR)
if [ "$(git rev-parse --git-dir)" != "$(git rev-parse --git-common-dir)" ] && ! grep -qE '^version = "[^"]*-[^"]*"' Cargo.toml; then
	echo "own-warp: $repo has no build tweak; run scripts/version-tweak.sh $(git branch --show-current) first" >&2
	exit 1
fi

target_dir="$(cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
src="$target_dir/debug/warp"
own_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
mkdir -p scratch
# Another checkout can rebuild the shared binary between our build and the copy: the copy must report our version
for attempt in 1 2 3; do
	cargo build --offline --bin warp
	if [ ! -x "$src" ]; then
		echo "own-warp: missing built binary at $src" >&2
		exit 1
	fi
	# Atomic replace: readers never see a partial binary (cp to a temp name, then mv).
	tmp="scratch/warp.tmp.$$"
	cp "$src" "$tmp"
	chmod +x "$tmp"
	# the whole version word: an untweaked 1.3.0 is no match for another checkout's 1.3.0-<branch>
	if [ "$("$tmp" version 2>/dev/null | head -1 | awk '{print $NF}')" = "$own_version" ]; then
		mv -f "$tmp" scratch/warp
		echo "$repo/scratch/warp"
		exit 0
	fi
	rm -f "$tmp"
	echo "own-warp: the shared binary was another checkout's build (attempt $attempt), building again" >&2
done
echo "own-warp: could not copy this checkout's build ($own_version)" >&2
exit 1
