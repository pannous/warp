#!/bin/bash
# card print-watch: a print before slow work shows in the playground while the work still runs, not once the run ends.
# Needs web/playground/build.sh (warp.wasm) and agent-browser; headless.
[ -n "${CI:-}" ] || { echo "browser tests run in CI only: skipped, Chrome is not started outside CI (user, 2026-10-09)"; exit 0; }
set -euo pipefail
PORT=${PORT:-8932}
REPO=$(cd "$(dirname "$0")/.." && pwd)
PROGRAM='print "Watch it turn blue."\ntotal = 0\nfor i in 0..300000000 { total += i % 7 }\ntotal'
browser() { agent-browser --session print-watch "$@"; }
WARP_BROWSER_TEST_PORT=$PORT python3 "$REPO/web/playground/test_in_browser.py" --serve >/dev/null 2>&1 &
server=$!
trap 'kill $server' EXIT
sleep 2
browser open "http://127.0.0.1:$PORT/web/playground/" >/dev/null
browser wait --fn 'window.playground && document.getElementById("status").textContent.match(/ready|ms$/)' >/dev/null
browser eval "window.finished = false; playground.runCode('$PROGRAM').then(() => window.finished = true); 0" >/dev/null
browser wait --fn 'document.getElementById("printed").textContent.includes("Watch it")' --timeout 20000 >/dev/null || { echo "FAIL: the print never showed"; exit 1; }
[ "$(browser eval 'window.finished')" = false ] || { echo "FAIL: the print showed only once the run ended (make the loop longer if it ran fast)"; exit 1; }
browser close >/dev/null
echo "ok: print-watch"
