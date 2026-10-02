#!/bin/bash
# runs every probe snippet in probes/globals with the warp binary built in /opt/cargo/warp-globals
cd "$(dirname "$0")/../.."
for f in probes/globals/${1:-}*.wasp; do printf '%-28s %-55s → ' "$(basename $f)" "$(cat $f)"; timeout 30 /opt/cargo/warp-globals/debug/warp "$f" 2>&1 | grep -v '^\s*$' | tail -2 | tr '\n' ' '; echo; done
