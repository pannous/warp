# pure-c-imports: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3052 pure emitter: wasm imports for from c declarations ([pure-c-imports](https://github.com/pannous/warp/issues/3052))
- Branch: `pure-c-imports` at `3d9802b7c`, 9 ahead / 254 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp FFI (parked 2026-10-23, handover notes/box/handover/Warp FFI.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: C imports in the pure lane (libc/libm/sqlite lines in lib/native, bool C types, a bytes foreign type); base of the ffi stack and of warp-host-integration; after a main merge test_ffi.sh failed 0/21 (every C call refused 'outside the core').

## Done
lib/native lines, c_imports.warp, the beside-compiler fix (3d9802b7c).

## Missing
Find why every C call is refused after a main merge, fix, rerun test_ffi on the merged tip, Ready.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
test_ffi.sh 0/21 after a main merge.

## Next step
Fix the refusal; it unblocks ffi-*, pass-owners-table and the warp-host-integration stack.
