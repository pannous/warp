#!/bin/bash
# Build the warp CLI (the Rust harness) and keep a private copy under scratch/warp: the shared cargo target dir's
# debug/warp is one file for every checkout, another checkout's build overwrites it.
# Usage (from repo root or anywhere): scripts/own-warp.sh
# Prints the absolute path of scratch/warp on success.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

target_dir="$(cargo metadata --offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
cargo build --offline --bin warp
mkdir -p scratch
# atomic replace: readers never see a partial binary
tmp="scratch/warp.tmp.$$"
cp "$target_dir/debug/warp" "$tmp"
chmod +x "$tmp"
mv -f "$tmp" scratch/warp
echo "$repo/scratch/warp"
