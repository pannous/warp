#!/bin/sh
# card firefox-hello-hang: a command Firefox never answers ends with the page's state and console, not a bare timeout.
# Usage: probes/firefox_hello_hang/unanswered.sh [url] (default the deployed playground)
URL=${1:-https://warp.pannous.com/}
REPOSITORY=$(cd "$(dirname "$0")/../.." && pwd)
printf '%s\n' "[\"open\", \"$URL\"]" '["eval", "new Promise(() => {})"]' '["close"]' |
	FIREFOX_COMMAND_SECONDS=5 node "$REPOSITORY/web/playground/firefox_driver.mjs"
