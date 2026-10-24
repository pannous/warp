# keys-of-empty: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #2976 keys of an empty map is [] in warp ([keys-of-empty](https://github.com/pannous/warp/issues/2976))
- Branch: `keys-of-empty` at `9561a80a2`, 3 ahead / 815 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/emitter.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: No owner: the card waits in Next for the planner to assign.
- State: emitter.warp: an untyped parameter widens to the kinds passed to it (keys of {}), plus dropping two twin probes where warp is right; the widening broke the self build.

## Done
The widening and the probe change; decision confirmed by the user.

## Missing
Land the probe part alone; fix the widening so self builds; emitter.warp conflict.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Self build broken by the widening (fold).

## Next step
Split: probes first, then a fixed widening; planner assigns.
