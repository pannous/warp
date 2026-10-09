#!/bin/bash
# Login tickets of warp-hosting (web/hosting/hosting.mjs, card deploy-opens-tab), against `wrangler dev` with its
# local KV: a ticket stores the program's address, refuses foreign addresses and bad tickets, and rides in the
# GitHub login's signed state. No browser (user, 2026-10-09: no local Chrome).
PORT=8896
TICKET=probe$(date +%s)abcdefghijklmn # local KV keeps values between runs
HOSTING="http://127.0.0.1:$PORT"
cd "$(dirname "$0")/../../web/hosting" || exit 1
wrangler dev --port $PORT --ip 127.0.0.1 --var SESSION_SECRET:probe --var GITHUB_CLIENT_ID:probe-client >/dev/null 2>&1 &
WRANGLER=$!
trap 'kill $WRANGLER' EXIT
for _ in $(seq 60); do curl -s "$HOSTING/" >/dev/null && break; sleep 1; done

failures=0
check() { # name, expected text, actual text
	if [[ "$3" == *"$2"* ]]; then echo "ok   $1"; else echo "FAIL $1: wanted $2, got $3"; failures=$((failures + 1)); fi
}
check "empty ticket" '{}' "$(curl -s "$HOSTING/ticket?ticket=$TICKET")"
check "program stored" '"stored":true' "$(curl -s -X POST "$HOSTING/ticket?ticket=$TICKET&program=https://warp-demo.pannous.workers.dev")"
check "program read" '"program":"https://warp-demo.pannous.workers.dev"' "$(curl -s "$HOSTING/ticket?ticket=$TICKET")"
check "foreign address refused" 'no deployed program' "$(curl -s -X POST "$HOSTING/ticket?ticket=$TICKET&program=https://evil.example.com")"
check "short ticket refused" 'no ticket' "$(curl -s "$HOSTING/ticket?ticket=short")"
check "error stored" '"error":"broken"' "$(curl -s -X POST "$HOSTING/ticket?ticket=${TICKET}x&error=broken" >/dev/null; curl -s "$HOSTING/ticket?ticket=${TICKET}x")"
redirect=$(curl -s -o /dev/null -w '%{redirect_url}' "$HOSTING/auth/github?origin=http://localhost:8080&ticket=$TICKET")
state=$(python3 -c 'import sys,urllib.parse,base64;s=urllib.parse.parse_qs(urllib.parse.urlparse(sys.argv[1]).query)["state"][0].split(".")[0];print(base64.urlsafe_b64decode(s+"="*(-len(s)%4)).decode())' "$redirect")
check "ticket in login state" "\"ticket\":\"$TICKET\"" "$state"
signed_state=$(python3 -c 'import sys,urllib.parse;print(urllib.parse.parse_qs(urllib.parse.urlparse(sys.argv[1]).query)["state"][0])' "$redirect")
check "refused login page" 'Login failed: access_denied' "$(curl -s "$HOSTING/callback?state=$signed_state&error=access_denied")"
check "refused login under the ticket" '"login":{"error":"access_denied"}' "$(curl -s "$HOSTING/ticket?ticket=$TICKET")"
check "bad ticket at login refused" 'no ticket' "$(curl -s "$HOSTING/auth/github?origin=http://localhost:8080&ticket=bad")"
exit $failures
