#!/bin/sh
# rebuild the warp lib of this copy right before linking: the shared target has one unhashed libwarp.rlib
cd "$(dirname "$0")/impl-text_work" && touch src/lib.rs && CARGO_BUILD_JOBS=2 cargo test --offline --test tests -- "$@" 2>&1 | grep -E "^error|^test |panicked|left:|right:|expected|test result|error\[" -A2
