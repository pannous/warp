#!/bin/bash
# runs every snippet (expected 6 / 5) through the warp CLI
cd "$(dirname "$0")"
while IFS= read -r line; do printf '%-60s => ' "$line"; /Users/me/.cargo/shared-target/debug/warp "$line" 2>&1 | tail -1; done < snippets.wasp
