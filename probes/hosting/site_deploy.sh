#!/bin/bash
# A browser program deployed through warp-hosting (card deploy-browser-site, notes/hosting.md "Browser programs"): the
# control Worker under wrangler dev, `warp deploy --hosted` of a paint sample as warp-repro-site, its page, script and
# module read on workers.dev, then removed. Needs scratch/warp (scripts/own-warp.sh), gh logged in, and
# CLOUDFLARE_API_TOKEN (Workers Scripts: Edit) in ~/.keys: wrangler's own OAuth token answers 401.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK="$ROOT/scratch/site_deploy"
PORT=8897
NAME=repro-site
HOSTING="http://localhost:$PORT"

mkdir -p "$WORK"
TOKEN=$(source ~/.keys > /dev/null 2>&1; echo "$CLOUDFLARE_API_TOKEN")
printf 'SESSION_SECRET=%s\nCLOUDFLARE_API_TOKEN=%s\n' "$(uuidgen)" "$TOKEN" > "$WORK/dev.vars"
(cd "$ROOT/web/hosting" && exec wrangler dev --port $PORT --inspector-port 9332 --env-file "$WORK/dev.vars") > "$WORK/wrangler.log" 2>&1 &
HOSTING_PID=$!
GITHUB=$(gh auth token)
remove() {
	curl -s -X DELETE -H "authorization: Bearer $GITHUB" "$HOSTING/deploy?name=$NAME" || true
	kill $HOSTING_PID
}
trap remove EXIT
until curl -s -o /dev/null "$HOSTING"; do sleep 1; done

cp "$ROOT/samples/filled_circle.warp" "$WORK/$NAME.warp"
URL=$(WARP_HOSTING=$HOSTING WARP_HOSTING_TOKEN=$GITHUB "$ROOT/scratch/warp" deploy --hosted "$WORK/$NAME.warp" | sed -n 's/^deployed: //p')
echo "deployed: $URL"
for _ in $(seq 90); do curl -sf -o /dev/null "$URL/canvas.js" && break; sleep 2; done
curl -s -D - "$URL/" | grep -iE "^(HTTP|content-type|cross-origin)|<script|data-warp-worker"
curl -s -o /dev/null -w "canvas.js %{http_code} %{content_type}\n" "$URL/canvas.js"
curl -s "$URL/app.wasm" | head -c 4 | xxd | head -1
