#!/bin/bash
# Serve a warp program and print status and body start of each path: probes/server_edits/curl_checks.sh <warp binary> <file.warp> <port>
# A path "POST /api/users Cy" posts the body Cy.
WARP=$1 PROGRAM=$2 PORT=$3
BODY_PREVIEW=120
rm -f "${PROGRAM%.warp}.database.sqlite"
WARP_RUNTIME_STUB=x "$WARP" serve "$PROGRAM" "$PORT" > /dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER' EXIT
until curl -s -o /dev/null "localhost:$PORT/"; do sleep 0.5; done

check() {
	local method=$1 path=$2 body=$3
	local answer
	answer=$(curl -s -X "$method" ${body:+-d "$body"} -w '\n%{http_code}' "localhost:$PORT$path")
	local status=${answer##*$'\n'}
	local content=${answer%$'\n'*}
	content=${content//$'\n'/ }
	echo "$method $path → $status ${content:0:$BODY_PREVIEW}"
}

check POST /rpc/user_count '[]'
check GET /api/users
check POST /api/users Cy
check GET /api/users
check GET /api/users/2
check GET /api/users/abc
check GET /api/users/9
check GET /api/newest
check GET /users/2
check GET /users/abc
check GET /nope
check GET /app.wasm
