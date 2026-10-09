#!/bin/bash
# samples/todo_app.warp under `warp serve`, checked with curl: create, toggle and list (as
# tests/web/test_served_forms.rs the_todo_sample_creates_toggles_and_lists does). Usage: check_served.sh [warp binary]
set -e
WARP=${1:-warp}
PORT=18648
cd "$(dirname "$0")/../.."
rm -f samples/todo_app.database.sqlite
"$WARP" serve samples/todo_app.warp $PORT > /dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER; rm -f samples/todo_app.database.sqlite' EXIT
until curl -s -o /dev/null localhost:$PORT/; do
	kill -0 $SERVER 2>/dev/null || { echo "FAIL $WARP serve samples/todo_app.warp exited before answering"; exit 1; }
	sleep 0.5
done
expect() {
	[ "$2" = "$3" ] && echo "ok   $1" || { echo "FAIL $1: $2, expected $3"; exit 1; }
}
expect "create" "$(curl -s -d title=tea localhost:$PORT/todos)" '{"Todo":{"title":"tea","done":false,"id":1}}'
expect "toggle" "$(curl -s -X POST localhost:$PORT/todos/1/toggle)" '{"Todo":{"title":"tea","done":true,"id":1}}'
expect "list" "$(curl -s localhost:$PORT/api/todos)" '[{"Todo":{"title":"tea","done":true,"id":1}}]'
expect "open" "$(curl -s localhost:$PORT/api/open)" '[]'
expect "page" "$(curl -s localhost:$PORT/ | grep -o '<p>0 open</p>')" '<p>0 open</p>'
