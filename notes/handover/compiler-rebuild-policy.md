# compiler-rebuild-policy: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3202 warp-host gives the compiler run the lib/ it… ([compiler-rebuild-policy](https://github.com/pannous/warp/issues/3202))
- Branch: `compiler-rebuild-policy` at `1f0233744`, 6 ahead / 201 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: scripts/self-check.sh)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Compiler pairs stored by an inputs hash (compiler-inputs.sh, rebuild.sh, self-check.sh) on top of the warp-host stack; one conflict in scripts/self-check.sh.

## Done
scripts/compiler-inputs.sh, rebuild.sh, the compiler store, WARP_LIB (P1); readied once at 1f0233744.

## Missing
Resolve scripts/self-check.sh against main, rerun the host tests, Ready again; contained in launcher-host-dispatch and scripts-off-cargo.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
None measured since the conflict.

## Next step
Merge main (self-check.sh), rerun tests, Ready; land before its two children.
