#!/usr/bin/env bash
# test/apps/check_app_samples.sh [<tree>]: the dogfood programs in samples/apps (card samples-dogfood-apps) run as plain
# programs and print what test/apps/expected/<name>.txt holds; exits 1 naming each that differs.
# test/apps/known_failures.txt lists "<name> <card>" for a program a known compiler bug breaks: it is reported as
# KNOWN (never silently skipped), and as FIXED once it passes, so the line can go
tree=$(cd "${1:-$(dirname "$0")/../..}" && pwd)
failed=0
known() { grep -E "^$1 " "$tree/test/apps/known_failures.txt" 2>/dev/null | cut -d' ' -f2-; }
for want in "$tree"/test/apps/expected/*.txt; do
	name=$(basename "$want" .txt)
	got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 TMPDIR=$(mktemp -d) timeout 300 "${WARP:-warp}" "samples/apps/$name.warp" 2>&1 | grep -v 'alternate stack')
	card=$(known "$name")
	if [ "$got" == "$(cat "$want")" ]; then
		if [ -n "$card" ]; then echo "FIXED samples/apps/$name.warp: drop its line ($card) from test/apps/known_failures.txt"; else echo "ok   samples/apps/$name.warp"; fi
	elif [ -n "$card" ]; then echo "KNOWN samples/apps/$name.warp ($card): $(echo "$got" | tail -1 | cut -c1-160)"
	else echo "FAIL samples/apps/$name.warp"; diff <(echo "$got") "$want" | head -20; failed=1; fi
done
exit $failed
