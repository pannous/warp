# ffi-sqlite3: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3088 all libraries in pure Warp over FFI ([ffi-libraries-roadmap](https://github.com/pannous/warp/issues/3088))
- Branch: `ffi-sqlite3` at `cdddd6d51`, 14 ahead / 329 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, lib/compiler/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp FFI (parked 2026-10-23, handover notes/box/handover/Warp FFI.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: sqlite3 in pure warp under warp-host (open_v2, exec, binds of every type, query_maps, errors); base of ffi-z and ffi-libc, conflicts in ffi.rs/main.rs/c_imports.warp.

## Done
48 C functions, positive checks; test_ffi_sqlite 20/20 before the positive rework.

## Missing
pure-c-imports first, main merge, rerun.

## How to test
Build warp-host from the merged tip, then tests/host/test_ffi.sh, test_ffi_sqlite.sh, test_ffi_z.sh, test_ffi_libc.sh (whichever the branch has) and the warp-only gate.

## Known failures
Not rerun after the positive rework.

## Next step
Land through ffi-libc (contains it).
