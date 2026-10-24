# launcher-host-dispatch: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3209 packaging/bin/warp sends run to warp-host ([launcher-host-dispatch](https://github.com/pannous/warp/issues/3209))
- Branch: `launcher-host-dispatch` at `d99fde44b`, 7 ahead / 201 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: scripts/self-check.sh)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: The launcher sends run, pure, self, compile, inline code and .wasm to warp-host (rebuild to scripts/rebuild.sh); on top of compiler-rebuild-policy, one scripts/self-check.sh conflict.

## Done
packaging/bin/warp dispatch.

## Missing
Resolve self-check.sh, check against decision launcher-not-yet, verify and Ready.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not verified (worker cut off).

## Next step
After compiler-rebuild-policy: merge main, verify, Ready.
