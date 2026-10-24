#!/bin/bash
# Screenshot of a self-contained WebGPU page in the browser-check Chrome on pannous.com (no browser on the Mac):
#   probes/ocean/screenshot_on_server.sh <page.html> <out.png> [query] [wait seconds]
# Same sandboxing as scripts/browser-dry-run.sh: the browsercheck user, a memory-capped low-priority scope.
set -euo pipefail
SERVER=pannous.com
RUN_DIR=/srv/browser-checks/runs/ocean
page=$1 out=$2 query=${3:-} wait_seconds=${4:-60}
ssh "$SERVER" "mkdir -p $RUN_DIR && chown browsercheck $RUN_DIR"
scp -q "$page" "$SERVER:$RUN_DIR/page.html"
ssh "$SERVER" "bash -s -- $(printf "%q " "$RUN_DIR" "$query" "$wait_seconds")" <<'REMOTE'
run_dir=$1 query=$2 wait_seconds=$3
chown browsercheck "$run_dir/page.html"
browser() {
	systemd-run --quiet --scope --uid=browsercheck --gid=browsercheck -p MemoryMax=2500M -p MemorySwapMax=0 -p CPUWeight=20 \
		env -i HOME=/srv/browser-checks USER=browsercheck LANG=C.UTF-8 PATH=/srv/browser-checks/npm/bin:/usr/bin:/bin \
		AGENT_BROWSER_ARGS=--no-sandbox,--enable-unsafe-webgpu,--enable-unsafe-swiftshader,--use-webgpu-adapter=swiftshader,--enable-features=Vulkan \
		timeout 300 nice -n 15 agent-browser --session ocean "$@"
}
browser set viewport 960 540
browser open "file://$run_dir/page.html?$query"
sleep "$wait_seconds"
browser eval "document.body.innerText.slice(0, 300)"
browser screenshot "$run_dir/shot.png"
browser close
REMOTE
scp -q "$SERVER:$RUN_DIR/shot.png" "$out"
echo "wrote $out"
