# pure-harness-loose-equal: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3168 pure harness: compare like is! ([pure-harness-loose-equal](https://github.com/pannous/warp/issues/3168))
- Branch: `pure-harness-loose-equal` at `f78a5e673`, 2 ahead / 264 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Code 2 (grok-warp-code-2; parked 2026-10-23, handover notes/box/handover/warp-code-2.md); waits for the Rust lane: that owner continues it when resumed (the planner reassigns it otherwise).
- State: Rust test harness: src/compiler/pure_tests.rs compares results as `is!` did (2 == 2.0, 'a' == "a"), removing about 196 false failures; Supervisor-approved, not measured.

## Done
pure_tests.rs +40/-2.

## Missing
A box build (own CARGO_TARGET_DIR via box-slot), before/after pure-suite counts (/workspace/harness196.json, /workspace/harness-cases), `todo ready` without --express.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not measured.

## Next step
Rust lane: build, measure, Ready.
