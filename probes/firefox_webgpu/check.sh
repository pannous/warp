#!/bin/sh
# Does headless Firefox (FIREFOX_PREFS) offer WebGPU on the page, in a Worker, and to a Worker's OffscreenCanvas?
# Runs in CI (card shaders-firefox): probes/firefox_webgpu/check.sh [url]
URL=${1:-https://warp.pannous.com/}
HERE=$(cd "$(dirname "$0")" && pwd)
CHECK=$(tr '\n' ' ' < "$HERE/check.mjs" | sed 's/"/\\"/g')
printf '["open", "%s"]\n["eval", "%s"]\n["messages"]\n["close"]\n' "$URL" "$CHECK" | node "$HERE/../../web/playground/firefox_driver.mjs"
