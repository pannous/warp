# pass-owners-table: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: draft card hDLGn0 "passes.warp names the pass order and each refused form's owner (pass-owners-table)" (Next)
- Branch: `pass-owners-table` at `36c13bceb`, 10 ahead / 254 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, core/emitter/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp FFI (parked 2026-10-23, handover notes/box/handover/Warp FFI.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: passes.warp names the pass order and each refused form's owner (pass_owners, owned_refusal) with tests/host/test_pass_owners.sh; on top of pure-c-imports, never built or tested.

## Done
Card pass-order-owners steps 1, 2 and 4.

## Missing
Supervisor's OK for the passes.warp change, pure-c-imports first, a build and test run.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not built or tested.

## Next step
After pure-c-imports: Supervisor's OK, then test and Ready.
