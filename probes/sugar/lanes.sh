#!/bin/bash
# runs every probe of probes/sugar/<pass>/ through the lanes: pure-only (lib read at run time), pure and self
# (the compiled builds at <lib>/../data); usage: [LIB=<lib>] probes/sugar/lanes.sh <pass> [lanes…]
pass=$1; shift
lanes=${*:-pure-only}
here=$(cd "$(dirname "$0")" && pwd)
lib=${LIB:-$(cd "$here/../../lib" && pwd)}
for probe in "$here/$pass"/*.warp; do
	echo "== $(basename "$probe" .warp): $(head -1 "$probe" | sed 's/.*Expected: //')"
	for lane in $lanes; do
		if [ "$lane" = pure-only ]; then
			out=$(WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_SELF_HOSTED=pure-only WARP_LIB="$lib" timeout 300 /Users/me/dev/bin/warp "$probe" 2>&1)
		else
			out=$(WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB="$lib" timeout 300 /Users/me/dev/bin/warp "$lane" "$probe" 2>&1)
		fi
		echo "  $lane: $(echo "$out" | grep -v -E '^(warning|[[:space:]]*[0-9]* \||[[:space:]]*\|)' | tr '\n' ' ' | cut -c1-300)"
	done
done
