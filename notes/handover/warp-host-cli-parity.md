# warp-host-cli-parity: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3207 warp-host CLI parity with warp: --fuel n… ([warp-host-cli-parity](https://github.com/pannous/warp/issues/3207))
- Branch: `warp-host-cli-parity` at `5651b154e`, 18 ahead / 199 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/warp_host/nodes.rs, core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: warp-host's command line matches warp's (--fuel, rationals shown as warp does); rest of the parity list unfinished; conflicts in nodes.rs and c_imports.warp.

## Done
--fuel, rational display.

## Missing
WARP_TIME_LIMIT, Error on stdout, WARP_EMIT_REFUSALS, pure/self/version words, wat, display 4 not 4.0; 3 host test lines pinning 4.0/5.0 need the user's OK.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not verified.

## Next step
Finish the parity list after warp-host-integration.
