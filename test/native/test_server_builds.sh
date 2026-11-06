#!/usr/bin/env bash
# The build kinds `warp build` detects from a program with routes (decision build-kinds-detected): its executable serves
# them (card server-build), and `warp build wagi` makes app.wasm answer the one request of its environment the WAGI way
# (card site-wagi-build). Run it with bash test/native/test_server_builds.sh; the warp under test is WARP (default the
# warp on PATH)
warp=${WARP:-warp}
stage=$(mktemp -d) || exit 1
server=
trap '[ -n "$server" ] && kill $server 2>/dev/null; rm -rf "$stage"' EXIT
failures=0
check() { # check <name> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $(printf '%q' "$2"), expected $(printf '%q' "$3")"; failures=$((failures + 1)); fi
}
printf '%s\n' 'use web' 'get "/hi" { "hi" }' 'get "/twice/:n:int" { n * 2 }' >"$stage/app.warp"

# the executable serves its routes, at the port WARP_PORT names (0: one the system chooses, printed as it serves)
(cd "$stage" && "$warp" build app.warp >"$stage/build.log" 2>&1)
if [ -x "$stage/app" ]; then
	WARP_PORT=0 "$stage/app" >"$stage/serving.log" 2>&1 &
	server=$!
	for _ in $(seq 50); do grep -q "serving http" "$stage/serving.log" && break; sleep 0.2; done
	address=$(grep -o "http://localhost:[0-9]*" "$stage/serving.log" | head -1)
	check "server: text" "$(curl -s "$address/hi")" "hi"
	check "server: typed parameter" "$(curl -s "$address/twice/21")" "42"
	check "server: no route" "$(curl -s -o /dev/null -w '%{http_code}' "$address/nowhere")" "404"
else
	echo "✗ server: no executable"; tail -3 "$stage/build.log"; failures=$((failures + 1))
fi

# the WAGI module answers the request of its environment, CGI-style: headers, a blank line, the body
(cd "$stage" && rm -f app.wasm && "$warp" build wagi app.warp >"$stage/wagi.log" 2>&1)
if [ -f "$stage/app.wasm" ]; then
	wagi() { REQUEST_METHOD=GET PATH_INFO=$1 QUERY_STRING= "$warp" "$stage/app.wasm" 2>/dev/null; }
	check "wagi: text" "$(wagi /hi)" "$(printf 'Status: 200\nContent-Type: text/plain; charset=utf-8\n\nhi')"
	check "wagi: json" "$(wagi /twice/4 | tail -1)" "8"
	check "wagi: no route" "$(wagi /nowhere | head -1)" "Status: 404"
else
	echo "✗ wagi: no app.wasm"; tail -3 "$stage/wagi.log"; failures=$((failures + 1))
fi
exit $((failures > 0))
