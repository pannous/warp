#!/usr/bin/env bash
# slow(reason, fix) (lib/os.warp, card slow-path-loud): a slow fallback says so once per reason on standard error, its
# fix named; `warp --strict` refuses it. The warp under test is WARP (default the one on PATH), its lib LIB
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
failures=0
check() { if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi; }
program='use os
slow("lists are copied", "use a GPU list")
slow("lists are copied", "use a GPU list")
slow("maps are copied", "use a record")
print "done"'
said=$(WARP_WINDOW=0 "$warp" --lib "$lib" "$program" 2>&1 >/dev/null)
check "each reason is said once, with its fix" "$(printf '%s\n' "$said" | sed 's/ ([^)]*)//')" $'slow path: lists are copied; fix: use a GPU list\nslow path: maps are copied; fix: use a record'
check "the output stays clean" "$(WARP_WINDOW=0 "$warp" --lib "$lib" "$program" 2>/dev/null)" "done"
check "--strict refuses it, at the call" "$(WARP_WINDOW=0 "$warp" --strict --lib "$lib" "$program" 2>&1 | grep -c 'slow path: lists are copied at [0-9:]*; fix: use a GPU list')" 1
[ "$failures" = 0 ] || exit 1
