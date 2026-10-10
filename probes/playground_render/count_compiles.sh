#!/bin/bash
# card playground-render: how many modules the playground compiles to show the tour example "fine updates" and to
# handle three clicks (each compiled module is posted to the page as {type: "module"}). Run from a checkout whose
# playground is built (web/playground/build.sh); prints "run: N, clicks: M".
[ -n "${CI:-}" ] || { echo "browser tests run in CI only: skipped, Chrome is not started outside CI (user, 2026-10-09)"; exit 0; }
cd "$(dirname "$0")/../.." || exit 1
SESSION=count-compiles
PORT=8791
python3 -m http.server $PORT --bind 127.0.0.1 >/dev/null 2>&1 &
server=$!
trap 'kill $server; agent-browser --session $SESSION close >/dev/null 2>&1' EXIT
sleep 1
b() { agent-browser --session $SESSION "$@"; }
b open "http://127.0.0.1:$PORT/web/playground/" >/dev/null
sleep 3
b eval 'worker.addEventListener("message", ({ data }) => { if (data.type === "module") window.compiled = (window.compiled ?? 0) + 1; }); "ok"' >/dev/null
b eval '(async () => { window.compiled = 0; await playground.chooseExample("fine updates"); while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50)); window.afterRun = window.compiled; window.compiled = 0; const button = document.getElementById("rendered").shadowRoot.querySelector("button"); for (let i = 0; i < 3; i++) { button.click(); await new Promise(done => setTimeout(done, 300)); } return "done"; })()' >/dev/null
sleep 2
b eval '`run: ${window.afterRun}, clicks: ${window.compiled}`'
