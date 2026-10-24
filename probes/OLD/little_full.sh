#!/bin/bash
# card little-full: ⛶ over the playground's paintings fills the screen with the last one at its own aspect ratio and
# leaves again. Needs web/playground/build.sh (warp.wasm) and agent-browser; headless.
[ -n "${CI:-}" ] || { echo "browser tests run in CI only: skipped, Chrome is not started outside CI (user, 2026-10-09)"; exit 0; }
set -euo pipefail
PORT=${PORT:-8931}
REPO=$(cd "$(dirname "$0")/.." && pwd)
browser() { agent-browser --session little-full "$@"; }
WARP_BROWSER_TEST_PORT=$PORT python3 "$REPO/web/playground/test_in_browser.py" --serve >/dev/null 2>&1 &
server=$!
trap 'kill $server' EXIT
sleep 2
browser open "http://127.0.0.1:$PORT/web/playground/#frames" >/dev/null
sleep 8
[ "$(browser eval '!document.getElementById("full-screen").hidden')" = true ] || { echo "FAIL: no ⛶ over the painting"; exit 1; }
browser click "#full-screen" >/dev/null
sleep 1
filled=$(browser eval 'const c = document.querySelector("#paintings canvas:last-child").getBoundingClientRect(); document.fullscreenElement?.id === "painted" && Math.abs(c.width / c.height - 2) < 0.01 && (c.width === innerWidth || c.height === innerHeight)')
[ "$filled" = true ] || { echo "FAIL: full screen does not show the painting as large as fits"; exit 1; }
browser click "#full-screen" >/dev/null
sleep 1
[ "$(browser eval 'document.fullscreenElement === null')" = true ] || { echo "FAIL: ⛶ again does not leave full screen"; exit 1; }
browser close >/dev/null
echo "ok: little-full"
