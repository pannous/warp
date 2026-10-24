# warp-host-integration: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3214 wasm-load + pure-c-imports + compiler-build… ([warp-host-integration](https://github.com/pannous/warp/issues/3214))
- Branch: `warp-host-integration` at `9af177982`, 13 ahead / 199 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Integration of wasm-load + pure-c-imports + compiler-build + main b84496b86; one c_imports.warp conflict now; the Integrator's 2026-10-23 test found every C call refused (same self-compiler breakage as main then).

## Done
The integration merge; readied 9af177982.

## Missing
pure-c-imports fixed, merge main, rerun test_host/test_ffi/beside with compilers rebuilt from the tip.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
test_host many ✗, ffi 8/19, beside 5/7 (2026-10-23 14:15, Integrator).

## Next step
After pure-c-imports: merge main, retest, Ready.
