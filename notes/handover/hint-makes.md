# hint-makes: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #1772 🐞 The hint makes zero sense and doesn't even show… ([hint-makes](https://github.com/pannous/warp/issues/1772))
- Branch: `hint-makes` at `c065da3c0`, 2 ahead / 3018 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: codex (old Rust work, parked; the user: old Rust): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Rust: the hint explains which `add` definition handles a list; old Rust work, 3000 commits behind, parked.

## Done
The hint change and a merge of an old main.

## Missing
Nothing planned: src/ is frozen; Supervisor once said merge as is, the user later called it old Rust.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Not built on today's main.

## Next step
Stays parked; land only in a Rust batch if the user asks.
