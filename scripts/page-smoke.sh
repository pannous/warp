#!/usr/bin/env bash
# scripts/page-smoke.sh [--built] <tree>: the playground page of <tree> runs PAGE_SMOKE as the native warp does, headless
# (one page check, as the Mac allows). Without --built it first builds the page's compiler from <tree>/lib, so a landing
# checks its own compilers before the push (card page-hello-ref-type: every landing check was green while the page's
# text reader no longer matched the runtime's $text and every shown value threw); after-push.sh passes --built for
# the playground it just refreshed. A host import the page lacks fails every sample here, hello first (canonical_text).
# Prints the page sweep's line; exits 1 unless every sample prints what the native warp prints.
set -uo pipefail
PAGE_SMOKE=(samples/hello/hello.warp samples/web/component_totals.warp)  # hello and markup
built=
[ "${1-}" = --built ] && { built=1; shift; }
tree=$(cd "${1:?usage: page-smoke.sh [--built] <tree>}" && pwd) || exit 2
scripts=$(cd "$(dirname "$0")" && pwd)
if [ -z "$built" ]; then
	mkdir -p "$tree/data"
	"$tree/web/playground/build.sh" --lib "$tree/lib" compiler >"$tree/data/page-smoke-build.log" 2>&1 ||
		{ echo "page smoke: the page's compiler did not build: $(tail -1 "$tree/data/page-smoke-build.log" | cut -c1-160)"; exit 1; }
fi
nice -n 10 "$scripts/samples.sh" "$tree" "${PAGE_SMOKE[@]}" >/dev/null
got=$(nice -n 10 "$scripts/samples-web.sh" "$tree" "${PAGE_SMOKE[@]}" 2>&1 | tail -1)
echo "page smoke: $got"
[[ $got == *"${#PAGE_SMOKE[@]} of ${#PAGE_SMOKE[@]} print"* ]]
