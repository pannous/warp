#!/bin/sh
# card firefox-hello-hang: a worker whose warp.wasm download stalls is started again after STALLED_START_MS (60 s) and
# the first example then shows; start probes/firefox_hello_hang/trickle_server.py --stall-once first (--stall: the
# second start stalls too, and the example fails naming the worker's stage instead of waiting for ever).
# Usage: probes/firefox_hello_hang/restart.sh [url]
URL=${1:-http://127.0.0.1:18603/web/playground/}
REPOSITORY=$(cd "$(dirname "$0")/../.." && pwd)
RUN='(async () => { await playground.chooseExample(\"hello\"); while (document.getElementById(\"status\").textContent === \"running…\") await new Promise(done => setTimeout(done, 50)); return `${document.getElementById(\"status\").textContent} | ${document.getElementById(\"value\").textContent} | ${JSON.stringify(playground.state())}`; })()'
{
	echo "[\"open\", \"$URL\"]"
	echo "[\"eval\", \"$RUN\"]"
	echo '["messages"]'
	echo '["close"]'
} | FIREFOX_COMMAND_SECONDS=240 node "$REPOSITORY/web/playground/firefox_driver.mjs"
