#!/bin/sh
# card error-undefined: samples/music.warp (or the file given) in the deployed playground, headless Firefox:
# probes/music_go/check.sh [file.warp]
[ -n "${CI:-}" ] || { echo "browser tests run in CI only: skipped, Firefox is not started outside CI (user, 2026-10-09)"; exit 0; }
PLAYGROUND="$(cd "$(dirname "$0")/../../web/playground" && pwd)"
# the run as one JSON text (a JSON5 command may hold it as is): the program's quotes stay escaped
RUN=$(python3 -c "import json, sys; print(json.dumps('playground.evaluate(%s).then(report => JSON.stringify(report).slice(0, 1500))' % json.dumps(open(sys.argv[1]).read())))" "${1:-samples/music.warp}")
node "$PLAYGROUND/firefox_driver.mjs" <<COMMANDS
['open', 'https://warp.pannous.com/']
['eval', 'new Promise(loaded => setTimeout(() => loaded(document.title), 5000))'] // the page may reload once for isolation
['eval', $RUN]
['messages']
['close']
COMMANDS
