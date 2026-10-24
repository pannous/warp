# scripts-off-cargo: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3211 self-check scripts use the shared binary and… ([scripts-off-cargo](https://github.com/pannous/warp/issues/3211))
- Branch: `scripts-off-cargo` at `f6c5a69fc`, 8 ahead / 201 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: scripts/self-check.sh)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Self-checks, rebuild.sh and the host/launcher tests use the shared warp, warp-host and compiler store, no per-tree cargo build; on top of compiler-rebuild-policy, one self-check.sh conflict.

## Done
box-self-check.sh puts the tree's pair beside its copy of the shared binary.

## Missing
self-check.sh conflict, verify, Ready.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not verified.

## Next step
After compiler-rebuild-policy: merge main, verify, Ready.
