#!/bin/bash
# exit 0 when `download <url>` returns the fetched content, 1 when it does not, 125 when the tree does not build
cd /Users/me/dev/angles/warp/scratch/bisect_wt || exit 125
export CARGO_TARGET_DIR=/opt/cargo/warp-bisect
cargo --offline test --no-run --test probe_dl >/dev/null 2>&1 || exit 125
cargo --offline test --test probe_dl >/dev/null 2>&1
