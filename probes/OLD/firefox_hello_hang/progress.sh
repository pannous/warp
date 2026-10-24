#!/bin/sh
# card firefox-hello-hang: what the playground shows beside its status while warp.wasm loads, once per second for
# SAMPLES seconds, in headless Firefox; start probes/firefox_hello_hang/trickle_server.py [--stall] first.
# Usage: probes/firefox_hello_hang/progress.sh [samples] [url]
SAMPLES=${1:-12}
URL=${2:-http://127.0.0.1:18603/web/playground/}
REPOSITORY=$(cd "$(dirname "$0")/../.." && pwd)
SAMPLE='new Promise(done => setTimeout(done, 1000)).then(() => { const loading = document.getElementById(\"loading\"); return `${loading.hidden ? \"(hidden)\" : loading.textContent} | ${document.getElementById(\"status\").textContent}`; })'
{
	echo "[\"open\", \"$URL\"]"
	for _ in $(seq "$SAMPLES"); do echo "[\"eval\", \"$SAMPLE\"]"; done
	echo '["close"]'
} | node "$REPOSITORY/web/playground/firefox_driver.mjs"
