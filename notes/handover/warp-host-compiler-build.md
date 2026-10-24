# warp-host-compiler-build: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3182 warp-host finds compiler-self.wasm and… ([warp-host-compiler-beside](https://github.com/pannous/warp/issues/3182))
- Branch: `warp-host-compiler-build` at `2efcab3f4`, 10 ahead / 254 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: crates/warp-runtime/src/warp_host/ffi.rs, crates/warp-runtime/src/warp_host/main.rs, lib/compiler/c_imports.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md) (inside warp-host-integration): that owner continues it when resumed (the planner reassigns it otherwise).
- State: The host build puts both compilers beside warp-host and warp-host prefers them over lib/../data; contained in warp-host-integration, cache-key and cli-parity; carries pure-c-imports.

## Done
Build step and lookup order.

## Missing
Lands through warp-host-integration.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Blocked by pure-c-imports' test_ffi failure.

## Next step
Land via warp-host-integration.
