# fix-route: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3045 launcher routes warp fix and beautify to… ([fix-route](https://github.com/pannous/warp/issues/3045))
- Branch: `fix-route` at `be75257fd`, 7 ahead / 596 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/prelude.warp, packaging/bin/warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md); the branch was parked: that owner continues it when resumed (the planner reassigns it otherwise).
- State: `warp fix` / `warp beautify` written in warp (lib/fix, ~860 lines) and the launcher routes to it; conflicts in lib/prelude.warp and packaging/bin/warp.

## Done
lib/fix/fix.warp, command.warp, launcher routing.

## Missing
Merge main (prelude.warp, packaging/bin/warp), probes/fix, rerun the launcher test.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Unknown after a main merge.

## Next step
Merge main and rerun; fix-in-lib is the older variant (archived).
