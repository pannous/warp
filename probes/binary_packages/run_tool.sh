#!/bin/bash
# warp runs a package's prebuilt <name>.wasm (src/package_tools.rs) and never builds into the shared cargo target dir.
# Usage: probes/binary_packages/run_tool.sh <warp binary>   (from the repository root)
warp=${1:?warp binary}
shared=$(sed -nE 's/^target-dir *= *"(.*)"/\1/p' ~/.cargo/config.toml)
before=$(ls -l "$shared"/*/uniscript "$shared"/*/libuniscript.rlib 2>/dev/null)
"$warp" tool uniscript '<:alpha> <:fracture A>' || exit 1
"$warp" tool uniscript check || exit 1
ls -la ~/.cache/warp/packages/ | grep uniscript
after=$(ls -l "$shared"/*/uniscript "$shared"/*/libuniscript.rlib 2>/dev/null)
[[ $before == "$after" ]] && echo "OK: $shared untouched" || { echo "❌ $shared changed"; exit 1; }
