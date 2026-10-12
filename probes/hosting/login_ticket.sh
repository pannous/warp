#!/bin/bash
# Login tickets of warp-hosting (web/hosting/hosting.mjs, card deploy-opens-tab), against `wrangler dev` with its
# local KV: a ticket stores the program's address, refuses foreign addresses and bad tickets, and rides in the
# GitHub login's signed state; the deploy window (card user-click) links the program and opens it once it answers
# (deploying_window.mjs runs its script in node). No browser (user, 2026-10-09: no local Chrome).
PORT=8896
TICKET=probe$(date +%s)abcdefghijklmn # local KV keeps values between runs
HOSTING="http://127.0.0.1:$PORT"
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
HOSTING_SCRATCH="$REPO/scratch/hosting"
mkdir -p "$HOSTING_SCRATCH"
cd "$(dirname "$0")/../../web/hosting" || exit 1
CLOUDFLARE_API_TOKEN=$(source ~/.keys 2>/dev/null; echo "$CLOUDFLARE_API_TOKEN")
wrangler dev --port $PORT --ip 127.0.0.1 --var SESSION_SECRET:probe --var GITHUB_CLIENT_ID:probe-client \
	--var CLOUDFLARE_API_TOKEN:"$CLOUDFLARE_API_TOKEN" >/dev/null 2>&1 &
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
# the deploy window (card user-click): the program's link as soon as the ticket knows it, opened once it answers
WAITING=${TICKET}w
check "deploying page waits" 'Deploying' "$(curl -s "$HOSTING/deploying?ticket=$WAITING")"
curl -s -X POST "$HOSTING/ticket?ticket=$WAITING&program=https://warp-demo.pannous.workers.dev" >/dev/null
check "deploying page links the program" '<a href="https://warp-demo.pannous.workers.dev">' "$(curl -s "$HOSTING/deploying?ticket=$WAITING")"
curl -s "$HOSTING/deploying?ticket=$WAITING" | sed -n 's/.*<script>//; s/<\/script>.*//; p' > "$HOSTING_SCRATCH/deploying.js"
check "deploying page script parses" 'parses' "$(node --check "$HOSTING_SCRATCH/deploying.js" 2>&1 && echo parses)"
check "deploying window opens the program" 'opened: https://warp-demo.pannous.workers.dev' "$(node "$REPO/probes/hosting/deploying_window.mjs" "$HOSTING" "$HOSTING_SCRATCH/deploying.js")"
check "deploying page names the failure" 'Not deployed: broken' "$(curl -s "$HOSTING/deploying?ticket=${TICKET}x")"
check "live program is up" '"up":true' "$(curl -s "$HOSTING/up?program=https://warp-demo.pannous.workers.dev")"
check "missing program is not up" '"up":false' "$(curl -s "$HOSTING/up?program=https://warp-nothing-here-zz.pannous.workers.dev")"
check "foreign address not probed" 'no deployed program' "$(curl -s "$HOSTING/up?program=https://evil.example.com")"
# the account's own Workers (warp-hosting itself, warp-assistant) are no program names to claim; the body is no valid
# module, so even a failing guard uploads nothing
not_wasm() { printf '\0asm\1\0\0\0garbage'; }
check "own Worker's name refused" 'taken' "$(not_wasm | curl -s -X POST --data-binary @- -H "authorization: Bearer $(gh auth token)" "$HOSTING/deploy?name=hosting")"
check "a free name passes the guard" 'CompileError' "$(not_wasm | curl -s -X POST --data-binary @- -H "authorization: Bearer $(gh auth token)" "$HOSTING/deploy?name=probe-free-$(date +%s)")"
exit $failures
