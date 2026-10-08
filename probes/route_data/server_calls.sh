#!/bin/sh
# The server calls a served page makes (P221), counted by counting_proxy.py. Needs a built CLI at scratch/warp and
# agent-browser. Expected:
# - app.warp (card route-data-cache): home, first, second, home, first, second POSTs route·data·0 for "/users/1" and
#   "/users/2" once each: home comes from the replies in the HTML, later visits from host-tasks.js routeDataReplies.
# - worker_app.warp (card ssr-worker-replies): a direct visit of /users/2 POSTs nothing; its program runs in a Worker
#   (site-worker.js), which answers from the replies the page sends it.
set -e
PORT=8395
PROXY=8396
SITE=scratch/route_data_calls
mkdir -p $SITE
browse() { agent-browser --session route-data-calls "$@"; }
shown() { browse eval 'document.querySelector("#warp-root").innerText.replace(/\n/g, " ")'; }

# serves probes/route_data/$1 through the proxy and visits $2, then follows the links named in the remaining arguments
visit() {
	program=$1 path=$2
	shift 2
	cp probes/route_data/$program $SITE/app.warp && rm -f $SITE/app.database.sqlite
	sqlite3 $SITE/app.database.sqlite "create table users(id integer primary key, name text); insert into users(name) values('Ann'),('Bo')"
	(cd $SITE && exec ../warp serve app.warp $PORT) > $SITE/serve.log 2>&1 &
	server=$!
	until curl -s localhost:$PORT/ > /dev/null; do sleep 1; done
	python3 probes/route_data/counting_proxy.py $PROXY $PORT > $SITE/calls.log &
	proxy=$!
	sleep 1
	browse open http://localhost:$PROXY$path > /dev/null
	sleep 6
	shown
	for link in "$@"; do
		browse find text "$link" click > /dev/null
		sleep 2
		shown
	done
	browse close > /dev/null
	kill $proxy $server
	echo "-- $program: $(grep -c POST $SITE/calls.log || true) server calls"
	grep POST $SITE/calls.log || true
}

visit app.warp / first second home first second
visit worker_app.warp /users/2
