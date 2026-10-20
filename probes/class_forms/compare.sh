#!/bin/sh
# The lowered program of each line of forms.md from the Rust parser and from parser.warp (the front lane): a line
# marked ✗ differs (card pure-class-forms). Run from a worktree: probes/class_forms/compare.sh [forms file]
here=$(cd "$(dirname "$0")" && pwd)
lib=$(cd "$here/../../lib" && pwd)
warp=${WARP:-/Users/me/dev/bin/warp}
forms=${1:-$here/forms.md}
lowered() { WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$lib "$@" 2>&1 | grep -v '^warning' | tail -1; }
grep -v '^#' "$forms" | while IFS= read -r code; do
	[ -z "$code" ] && continue
	rust=$(lowered "$warp" lower "$code")
	warp_side=$(WARP_SELF_HOSTED=front lowered "$warp" lower "$code")
	if [ "$rust" = "$warp_side" ]; then echo "✓ $code"; else printf '✗ %s\n  rust: %s\n  warp: %s\n' "$code" "$rust" "$warp_side"; fi
done
