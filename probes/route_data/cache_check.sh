#!/bin/sh
# card route-data-cache: following links back and forth asks each route-data request once.
# Needs a built CLI at scratch/warp and agent-browser. Expected rpc fetch counts: first 1, second 2, then 2 throughout
# (home comes from the rendered page's warp-replies, the later visits from host-tasks.js routeDataReplies).
set -e
PORT=8391
SITE=scratch/route_data_cache
mkdir -p $SITE && cp probes/route_data/app.warp $SITE/ && rm -f $SITE/app.database.sqlite
sqlite3 $SITE/app.database.sqlite "create table users(id integer primary key, name text); insert into users(name) values('Ann'),('Bo')"
(cd $SITE && exec ../warp serve app.warp $PORT) > $SITE/serve.log 2>&1 &
SERVER=$!
trap 'kill $SERVER' EXIT
until curl -s localhost:$PORT/ > /dev/null; do sleep 1; done
browse() { agent-browser --session route-data-cache "$@"; }
browse open http://localhost:$PORT/ > /dev/null
sleep 3
for link in first second home first second; do
	browse find text "$link" click > /dev/null
	sleep 2
	browse eval 'document.querySelector("#warp-root").innerText.replace(/\n/g, " ") + " | rpc fetches: " + performance.getEntriesByType("resource").filter(entry => entry.name.includes("rpc")).length'
done
browse close > /dev/null
