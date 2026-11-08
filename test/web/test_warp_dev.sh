#!/usr/bin/env bash
# `warp dev page.warp port` (card dev-page) serves the page's site with web/playground/dev.js in it and its state at
# /warp-dev/state: a change to the page is the next version, a failing build its error while the last good build
# serves on. Headless: it opens no browser. Run it with bash test/web/test_warp_dev.sh; the warp under test is WARP
# (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
PORT=${WARP_DEV_TEST_PORT:-8791}
WAIT_SECONDS=120
status=0
folder=$(mktemp -d)
page=$folder/page.warp
printf 'div{ h1{"Fruit"} }\n' > "$page"
"$warp" dev "$page" "$PORT" > "$folder/dev.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null; rm -rf "$folder"' EXIT
url=http://localhost:$PORT
state() { curl -s "$url/warp-dev/state"; }
waited_for() {
	local deadline=$((SECONDS + WAIT_SECONDS))
	until [[ $(state) == *"$1"* ]] || [ $SECONDS -ge $deadline ]; do sleep 0.5; done
	state
}
says() {
	local what=$1 said=$2 expected=$3
	case $said in
		*"$expected"*) echo "✓ $what" ;;
		*) echo "✗ $what: expected …$expected…, got: ${said:0:300}"; status=1 ;;
	esac
}
says "the first build is version 1" "$(waited_for '"version":1')" '"version":1'
says "the page carries dev.js" "$(curl -s "$url/")" "warp-dev/state"
says "the page is the site's" "$(curl -s "$url/")" "web/playground/site.js"
says "isolated" "$(curl -s -D - -o /dev/null "$url/")" "Cross-Origin-Embedder-Policy: require-corp"
module_bytes=$(curl -s "$url/app.wasm" | head -c 4 | od -An -c | tr -d ' ')
says "the module served whole" "$module_bytes" '\0asm'
sleep 1.1  # a newer modification time than the first build's stamp
printf 'div{ h1{"Pears"} }\n' > "$page"
says "a change is the next version" "$(waited_for '"version":2')" '"version":2'
sleep 1.1
printf 'div{ h1{"Pears" }\n' > "$page"
deadline=$((SECONDS + WAIT_SECONDS))
until [[ $(state) != *'"error":""'* ]] || [ $SECONDS -ge $deadline ]; do sleep 0.5; done
failed=$(state)
says "a failing build is the page's error" "$failed" '"version":2'
[[ $failed == *'"error":""'* ]] && { echo "✗ the failure has no error: $failed"; status=1; }
says "the last good build serves on" "$(curl -s "$url/")" "warp-dev/state"
exit $status
