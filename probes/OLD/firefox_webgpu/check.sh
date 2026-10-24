#!/bin/sh
# Does headless Firefox (FIREFOX_PREFS) offer WebGPU on the page, in a Worker, and to a Worker's OffscreenCanvas?
# The playground reloads itself once for cross-origin isolation, so the page is opened twice.
# Runs in CI (card shaders-firefox): probes/firefox_webgpu/check.sh [url]
URL=${1:-https://warp.pannous.com/}
HERE=$(cd "$(dirname "$0")" && pwd)
node -e '
const [url, check] = process.argv.slice(1);
for (const command of [["open", url], ["eval", "new Promise(done => setTimeout(done, 3000))"], ["open", url], ["eval", require("fs").readFileSync(check, "utf8")], ["eval", "JSON.stringify(self.probeSteps)"], ["messages"], ["close"]]) console.log(JSON.stringify(command));
' "$URL" "$HERE/check.mjs" | node "$HERE/../../web/playground/firefox_driver.mjs" | node -e '
require("readline").createInterface({ input: process.stdin }).on("line", line => { const value = JSON.parse(line); console.log(typeof value === "string" ? value : JSON.stringify(value)); });
'
