# compiler-to-core-prep: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3196 Move lib/compiler into a core folder outside lib ([compiler-to-core](https://github.com/pannous/warp/issues/3196))
- Branch: `compiler-to-core-prep` at `351a21d45`, 3 ahead / 228 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/RUNTIME.md, core/emitter/c_imports.warp, core/compiler.warp, core/emitter/c_functions.warp, core/extensions/README.md …)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: The core/ layout flip as a script (scripts/move-compiler-to-core.sh) plus a dry-run move of lib/compiler to core (345 files); the branch itself can't be merged, the script is rerun on the main of the day.

## Done
move-compiler-to-core.sh, files.txt/branches.txt (in /workspace/cow/scratch/compiler-to-core/), one dry run.

## Missing
Rerun the script on the day's main as one sweep, drop its run_library edit, self-check (card compiler-to-core).

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
Conflicts everywhere by design (RUNTIME.md, compiler.warp, README.md …).

## Next step
Planned sweep on a quiet main; never merge this tip.
