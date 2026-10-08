#!/bin/bash
# card single-page: a site with routes is one page; a deep link (/users/2) gets it as 404.html (as GitHub Pages serves
# it, here from warp dev), its scripts load through <base href="/"> and its router shows the route of the path, in a
# headless browser (agent-browser). Usage: probes/site/deep_link_in_browser.sh [warp binary]
cd "$(dirname "$0")/../.." || exit 1
WARP=${1:-warp}
PORT=8932
"$WARP" dev probes/site/routes.warp "$PORT" > /dev/null &
SERVER=$!
trap 'kill $SERVER; agent-browser close >/dev/null 2>&1' EXIT
sleep 3
failures=0
expect() { # what, actual, expected
	if [[ "$2" != "$3" ]]; then echo "FAIL $1: '$2', expected '$3'"; failures=$((failures + 1)); fi
}
heading() { agent-browser eval 'document.querySelector("#warp-root h1, #warp-root p")?.textContent' | tail -1; }
shown_at() { # path, expected heading: waits up to 10 s for the page's router
	agent-browser open "http://127.0.0.1:$PORT$1" >/dev/null 2>&1
	for _ in $(seq 20); do [[ "$(heading)" == "$2" ]] && break; sleep 0.5; done
	expect "$1" "$(heading)" "$2"
}

shown_at "/users/2" '"User Bo"'
shown_at "/nowhere/at/all" '"not found"'
shown_at "/" '"Home"'
[[ $failures -eq 0 ]] && echo "deep links in the browser: ok"
exit $failures
