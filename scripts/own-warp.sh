#!/bin/bash
# Build the warp CLI and keep a private copy under scratch/warp.
# The shared cargo target dir's debug/warp is one file for every checkout: another worktree's build overwrites it.
# Usage (from repo root or anywhere): scripts/own-warp.sh
# Prints the absolute path of scratch/warp on success.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

# Optional: warn when a worktree still has the plain shared version (notes/build_speed.md).
if [[ "$repo" == *.worktrees.noindex/* ]]; then
	version="$(python3 -c '
import re, pathlib
text = pathlib.Path("Cargo.toml").read_text()
m = re.search(r"(?m)^version\s*=\s*\"([^\"]+)\"", text)
print(m.group(1) if m else "")
')"
	if [ "$version" = "0.1.1" ]; then
		name="$(basename "$repo")"
		# semver pre-release: alphanumeric and hyphen
		safe="$(printf '%s' "$name" | tr -c 'A-Za-z0-9-' '-')"
		echo "own-warp: Cargo.toml version is still 0.1.1 in worktree '$name'; set version = \"0.1.1-$safe\" (uncommitted) so this copy does not share lib artifacts" >&2
	fi
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
	if "$tmp" version 2>/dev/null | grep -qF "$own_version"; then
		mv -f "$tmp" scratch/warp
		echo "$repo/scratch/warp"
		exit 0
	fi
	rm -f "$tmp"
	echo "own-warp: the shared binary was another checkout's build (attempt $attempt), building again" >&2
done
echo "own-warp: could not copy this checkout's build ($own_version)" >&2
exit 1
