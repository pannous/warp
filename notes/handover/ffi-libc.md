# ffi-libc: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3088 all libraries in pure Warp over FFI ([ffi-libraries-roadmap](https://github.com/pannous/warp/issues/3088))
- Branch: `ffi-libc` at `02a9f22f4`, 31 ahead / 254 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/c_calls.rs, crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, crates/warp-runtime/src/warp_host/nodes.rs, core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp FFI (parked 2026-10-23, handover notes/box/handover/Warp FFI.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: libc in pure warp over FFI (memory, files, process, time, c_errno, strerror) on top of ffi-z, ffi-sqlite3 and pure-c-imports; conflicts with main in Rust files (c_calls.rs, ffi.rs, main.rs, nodes.rs).

## Done
libc foreign lines, c_errno, hand-linked shims for rust-lane modules; test_ffi_libc 10/11 last run.

## Missing
pure-c-imports' bug fixed first (every C call refused after a main merge), main merge, test_ffi on the merged tip.

## How to test
Build warp-host from the merged tip, then tests/host/test_ffi.sh, test_ffi_sqlite.sh, test_ffi_z.sh, test_ffi_libc.sh (whichever the branch has) and the warp-only gate.

## Known failures
test_ffi_libc 10/11 (c_errno, likely a stale compiler).

## Next step
Land after pure-c-imports, ffi-sqlite3, ffi-z (top of that stack).
