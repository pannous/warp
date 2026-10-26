# ffi-z: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3088 all libraries in pure Warp over FFI ([ffi-libraries-roadmap](https://github.com/pannous/warp/issues/3088))
- Branch: `ffi-z` at `06b1a8852`, 24 ahead / 329 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/c_calls.rs, crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, crates/warp-runtime/src/warp_host/nodes.rs, core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp FFI (parked 2026-10-23, handover notes/box/handover/Warp FFI.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: zlib in pure warp over FFI with byte lists of any size (1 MiB round trips); contains ffi-sqlite3, contained in ffi-libc.

## Done
buffer/written results, c_write/c_load/c_store; test_ffi_z 22/22 before the positive rework.

## Missing
pure-c-imports first, main merge, rerun.

## How to test
Build warp-host from the merged tip, then tests/host/test_ffi.sh, test_ffi_sqlite.sh, test_ffi_z.sh, test_ffi_libc.sh (whichever the branch has) and the warp-only gate.

## Known failures
Not rerun after the positive rework.

## Next step
Land through ffi-libc.
