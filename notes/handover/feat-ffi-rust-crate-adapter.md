# feat/ffi-rust-crate-adapter: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3091 general Rust crate adapter: use any crate from… ([ffi-rust-crate-adapter](https://github.com/pannous/warp/issues/3091))
- Branch: `feat/ffi-rust-crate-adapter` at `e48d45083`, 8 ahead / 436 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: grok-warp-crates (parked 2026-10-23; Supervisor: low priority): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Call Rust crates from warp (`use crate "regex@1"`, generated shims, bytes as pointer+length; test_crates 20/20 when parked); conflicts in crates/warp-runtime ffi.rs and core c_imports.warp.

## Done
Shim generation, crates cache, regex/sha2/crc32fast, tests/host/test_crates.sh.

## Missing
Merge main (ffi.rs, c_imports.warp), take ffi-z's byte lists, on-demand builds.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
test_crates 20/20 at the tip before main moved; unknown after a main merge.

## Next step
Waits for the Rust lane and pure-c-imports.
