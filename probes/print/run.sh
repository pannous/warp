#!/bin/bash
# runs every probe snippet in probes/print with the warp binary built in /opt/cargo/warp-globals
cd "$(dirname "$0")/../.."
for f in probes/print/${1:-}*.wasp; do printf '%-32s %-50s → ' "$(basename $f)" "$(cat $f)"; timeout 30 /opt/cargo/warp-globals/debug/warp "$f" 2>&1 | grep -v '^\s*$' | tail -2 | tr '\n' ' '; echo; done
