#!/bin/sh
# Runs every element-binding probe snippet with a build of the checkout given as $1 (default: this repo)
probes="$(cd "$(dirname "$0")" && pwd)"
checkout="${1:-$probes/../..}"
target="${CARGO_TARGET_DIR:-/opt/cargo/warp-elements}"
(cd "$checkout" && CARGO_TARGET_DIR="$target" cargo --offline build --all-features 2>&1 | grep -E "^error" -A7)
for snippet in "$probes"/*.wasp; do
	printf "== %s: " "$(basename "$snippet")"
	timeout 10 "$target/debug/warp" "$snippet" 2>&1 | grep -v "hint\|preferred\|use #" | tail -1
done
