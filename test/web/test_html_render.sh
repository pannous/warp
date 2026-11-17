#!/usr/bin/env bash
# A program ending in an HTML element renders it, with no `use` (decision html-render-mode): the playground shows the
# page, natively warp prints its HTML; a value of no element prints as before. Run it with bash
# test/web/test_html_render.sh; the warp under test is WARP (default the warp on PATH), a tree's WARP=<tree>/data/gate/warp
source "$(dirname "$0")/../native/checks.sh"
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT

ran() { # ran <name> <code>: the code as a file, run, its output without hints
	printf '%s\n' "$2" >"$stage/$1.warp"
	"$warp" "$stage/$1.warp" 2>&1 | grep -v '^hint:'
}

check "an element is its page" "$(ran element 'div{ h1{ "hi" } }')" "<div><h1>hi</h1></div>"
check "elements in a group" "$(ran group '[li{ "a" } li{ "b" }]')" "<li>a</li><li>b</li>"
check "another value prints as before" "$(ran plain 'x = {a: 1}
x.a + 1')" "2"
exit $((failures > 0))
