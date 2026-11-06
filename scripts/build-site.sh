#!/bin/sh
# scripts/build-site.sh <app.warp> [<folder>]: the program as a static site, any static host serves it (the folder
# defaults to site/ beside the program). app.wasm is `warp build --wasm`'s whole module (app.full.wasm: the trimmed one
# drops exports the page calls; app.mod.wasm's exact references load only in wasmtime). The page runs it in a Worker with the
# playground's host (web/playground/site.js, site-worker.js, driver.js), its markup rendered by lib/markup.warp
# page_render, its handlers run by page_event, a serving program's routes answering the page's links and forms.
# The folder mirrors the repository's layout (web/playground/ holds the scripts), so the program's file reads resolve
# against the site's root as they do against the repository in the playground.
set -eu

REPOSITORY=$(cd "$(dirname "$0")/.." && pwd)
PLAYGROUND="$REPOSITORY/web/playground"
SCRIPT_FOLDER="web/playground"
# the page's scripts, in order (site.js needs the others)
PAGE_SCRIPTS="markup.js markup-transitions.js markup-forms.js canvas.js task-workers.js host-routes.js site.js"
# the program's Worker and its task Workers (site-worker.js, task-worker.js importScripts)
WORKER_SCRIPTS="imports.js js-members.js std.js host.js values.js host-files.js host-sound.js
host-hashes.js host-tasks.js host-foreign.js host-sqlite.js host-compiler.js host-routes.js host-gpu.js host-timers.js
host-random.js host-media.js components.js wasi.js driver.js site-worker.js task-worker.js"
LIBRARIES="lib/libc.wasm"

source=${1:?usage: scripts/build-site.sh <app.warp> [<folder>]}
folder=${2:-$(dirname "$source")/site}
stem=${source%.warp}
title=$(basename "$stem")

warp build --wasm "$source"
# the page's first markup, prerendered: what the program shows once it ran (`warp run` serves no routes), so the page
# shows it before its Worker starts; the Worker's first render morphs it in place
shown=$(WARP_NO_WINDOW=1 warp run "$source" </dev/null 2>/dev/null | tail -1) || shown=""
case $shown in "<"*) ;; *) echo "build-site: no markup to prerender, the page shows its markup once its Worker runs" >&2; shown="" ;; esac
mkdir -p "$folder/$SCRIPT_FOLDER/lib"
mv "$stem.full.wasm" "$folder/app.wasm"
rm -f "$stem.wasm" "$stem.mod.wasm" "$stem"
for script in $PAGE_SCRIPTS $WORKER_SCRIPTS $LIBRARIES; do cp "$PLAYGROUND/$script" "$folder/$SCRIPT_FOLDER/$script"; done
# a static host gives no cross-origin isolation: this service worker adds it, registered from the site's root
cp "$PLAYGROUND/coi-serviceworker.js" "$folder/"

{
	printf '<!doctype html>\n<html>\n<head>\n<meta charset="utf-8">\n'
	printf '<meta name="viewport" content="width=device-width, initial-scale=1">\n<link rel="icon" href="data:,">\n'
	printf '<title>%s</title>\n</head>\n<body>\n' "$title"
	printf '<div id="warp-root">%s</div>\n' "$shown"
	for script in $PAGE_SCRIPTS; do printf '<script src="%s/%s"></script>\n' "$SCRIPT_FOLDER" "$script"; done
	printf '</body>\n</html>\n'
} > "$folder/index.html"

# the files the site has (host-files.js isUnserved): a read of any other fails at once instead of asking the host
files=$(cd "$folder" && find . -type f ! -name served-files.js | sed 's|^\./\(.*\)|"\1"|' | sort | paste -sd, -)
echo "const SERVED_FILES = new Set([$files]);" > "$folder/$SCRIPT_FOLDER/served-files.js"
echo "built $folder (open it from a static server: index.html)"
