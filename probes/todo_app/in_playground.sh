#!/bin/bash
# runs a warp file in the local playground (python3 -m http.server $PORT in the repository root, web/playground/build.sh
# done) and prints its status, value, rendered page and diagnostics
PORT=${PORT:-18660}
FILE=$1
code=$(python3 -c 'import json,sys; print(json.dumps(open(sys.argv[1]).read()))' "$FILE")
browse() { agent-browser --session todo5 "$@"; }
if [[ "$(browse get url 2>/dev/null)" != *"localhost:$PORT/web/playground/"* ]]; then
	browse open "http://localhost:$PORT/web/playground/" > /dev/null
	sleep 10
fi
browse eval "document.querySelector('.CodeMirror').CodeMirror.setValue($code); document.getElementById('run').click(); 'ran'" > /dev/null
sleep ${WAIT:-4}
browse eval '["status: " + document.getElementById("status").textContent, "value: " + document.getElementById("value").textContent, "page: " + (document.getElementById("rendered").shadowRoot?.innerHTML ?? "").replace(/<style>[^]*?<\/style>/g, "").slice(0, 2000), "diagnostics: " + [...document.querySelectorAll("#diagnostics li")].map(item => item.textContent).join(" | ")].join("\n")' | python3 -c 'import json,sys; print(json.loads(sys.stdin.read()))'
