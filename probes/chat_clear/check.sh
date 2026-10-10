#!/bin/sh
# card agent-ask: the chat's ✕ shows once the chat has anything and clears it; headless Firefox (no Chrome on this Mac)
# over the playground served as static files: probes/chat_clear/check.sh
PORT=$(python3 -c "import socket; s = socket.socket(); s.bind(('', 0)); print(s.getsockname()[1])") # a free one
PLAYGROUND="$(cd "$(dirname "$0")/../../web/playground" && pwd)"
python3 -m http.server $PORT --directory "$PLAYGROUND" >/dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER' EXIT
sleep 1
SHOWN='getComputedStyle(document.getElementById("chat-clear")).display'
# the commands are JSON5 (firefox_driver.mjs): single quotes around JavaScript with double quotes, comments
node "$PLAYGROUND/firefox_driver.mjs" <<COMMANDS
['open', 'http://localhost:$PORT/index.html']
['eval', 'new Promise(loaded => setTimeout(() => loaded(document.title), 3000))'] // the page may reload once for isolation
// hidden while the chat is empty, shown with a question, gone with it after a click
['eval', 'document.getElementById("ask").click(); $SHOWN']
['eval', 'document.getElementById("chat-log").append(Object.assign(document.createElement("div"), {className: "chat-question", textContent: "why?"})); $SHOWN']
['eval', 'document.getElementById("chat-clear").click(); [document.getElementById("chat-log").childElementCount, $SHOWN].join(" ")',]
['close']
COMMANDS
