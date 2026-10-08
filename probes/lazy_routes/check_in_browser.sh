#!/bin/bash
# card web-bundle: a site split by route (src/route_split.rs) in a headless browser (agent-browser): the first page
# loads only its own route's module, a link loads the next route's module and shows it, the back button shows the
# route again without loading it twice; after each route change the focus is on the route's heading, else the root. Usage: probes/lazy_routes/check_in_browser.sh [warp binary]
cd "$(dirname "$0")/../.." || exit 1
WARP=${1:-warp}
PORT=8931
WORK=scratch/lazy_routes
SITE=$WORK/three_routes-site
rm -rf "$WORK" && mkdir -p "$WORK" && cp probes/lazy_routes/three_routes.warp "$WORK/"
"$WARP" build --site "$WORK/three_routes.warp" || exit 1
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$SITE" 2> "$WORK/requests.log" &
SERVER=$!
trap 'kill $SERVER; agent-browser close >/dev/null 2>&1' EXIT
sleep 1
failures=0
shown() { agent-browser eval 'document.querySelector("#warp-root h1, #warp-root p")?.textContent' | tail -1; }
focused() { agent-browser eval '(document.activeElement.id || document.activeElement.tagName) + " " + document.activeElement.getAttribute("tabindex")' | tail -1; }
fetched() { grep -c "GET /$1 " "$WORK/requests.log"; }
expect() { # what, actual, expected
	if [[ "$2" != "$3" ]]; then echo "FAIL $1: '$2', expected '$3'"; failures=$((failures + 1)); fi
}

agent-browser open "http://127.0.0.1:$PORT/" >/dev/null && sleep 1
expect "home" "$(shown)" '"Home"'
expect "primes module before the link" "$(fetched app-route-1.wasm)" 0
agent-browser click 'a[href="/primes"]' >/dev/null && sleep 1
expect "primes" "$(shown)" '"primes: 2 3 5 7 11 13 17 19 23 29"'
expect "primes module after the link" "$(fetched app-route-1.wasm)" 1
expect "focus without a heading" "$(focused)" '"warp-root -1"'
agent-browser click 'a[href="/fib"]' >/dev/null && sleep 1
expect "fib" "$(shown)" '"fib 15 = 610"'
agent-browser back >/dev/null && sleep 1
expect "back to primes" "$(shown)" '"primes: 2 3 5 7 11 13 17 19 23 29"'
expect "primes module loaded once" "$(fetched app-route-1.wasm)" 1
agent-browser click 'a[href="/"]' >/dev/null && sleep 1
expect "focus on the heading" "$(focused)" '"H1 -1"'
[[ $failures -eq 0 ]] && echo "lazy routes in the browser: ok"
exit $failures
