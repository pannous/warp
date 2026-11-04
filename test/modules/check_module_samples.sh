#!/usr/bin/env bash
# test/modules/check_module_samples.sh [<tree>]: the samples combining modules (card lib-docs-samples) run and print
# what test/modules/module_samples/<sample>.txt holds; exits 1 naming each that differs
tree=$(cd "${1:-$(dirname "$0")/../..}" && pwd)
failed=0
for want in "$tree"/test/modules/module_samples/*.txt; do
	name=$(basename "$want" .txt)
	got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 timeout 120 "${WARP:-warp}" "samples/modules/$name.warp" 2>/dev/null | grep -v 'alternate stack')
	if [ "$got" == "$(cat "$want")" ]; then echo "ok   samples/modules/$name.warp"; else echo "FAIL samples/modules/$name.warp"; diff <(echo "$got") "$want" | head -20; failed=1; fi
done
exit $failed
