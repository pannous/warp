# type-zoned: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #1477 type of a zoned time with a negative offset ([type-zoned](https://github.com/pannous/warp/issues/1477))
- Branch: `type-zoned` at `f59c127e1`, 2 ahead / 1694 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/emitter/emitter.warp, core/runtime/durations.warp, core/runtime/extended/times.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Code 3 (grok-warp-code-3; parked 2026-10-23, handover notes/box/handover/Warp Code 3.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: A time's negative amounts (zones west of UTC, durations back in time) keep their kind; written before runtime-split, which landed, so it must be redone on main's runtime/extended files.

## Done
The fix in emitter.warp/durations.warp/times.warp (old layout) plus a Rust test.

## Missing
Redo on main's runtime/extended files, tests.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Conflicts with main in three files.

## Next step
Redo on main (Warp Code 3 listed it after runtime-split).
