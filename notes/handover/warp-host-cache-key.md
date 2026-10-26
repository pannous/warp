# warp-host-cache-key: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3208 warp-host cache key keyed by what the compile… ([warp-host-cache-key](https://github.com/pannous/warp/issues/3208))
- Branch: `warp-host-cache-key` at `893d32b07`, 16 ahead / 199 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: warp-host keys its kept compiler values by what the compile reads from lib/; top of the warp-host-integration stack, one c_imports.warp conflict; carries pure-c-imports.

## Done
Key = what compile reads.

## Missing
pure-c-imports' test_ffi failure fixed, main merge, verify, Ready; Rust lane.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not verified.

## Next step
After warp-host-integration.
