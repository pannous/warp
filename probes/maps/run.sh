#!/bin/bash
# runs every probe snippet in probes/maps with the warp binary built in /opt/cargo/warp-maps
cd "$(dirname "$0")/../.."
for f in probes/maps/${1:-}*.warp; do printf '%-40s ' "$(basename $f)"; timeout 30 /opt/cargo/warp-maps/debug/warp "$f" 2>&1 | grep -v '^\s*$' | tail -1; done
