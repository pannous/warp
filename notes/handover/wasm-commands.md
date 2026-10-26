# wasm-commands: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #2962 standalone wasm warp has the full native command… ([wasm-commands](https://github.com/pannous/warp/issues/2962))
- Branch: `wasm-commands` at `54756a917`, 11 ahead / 264 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/compiler.warp)
- Kind: touches Rust (src/, crates/ or *.rs): waits for the Rust lane
- Owner: Warp Code 3 (grok-warp-code-3; parked 2026-10-23, handover notes/box/handover/Warp Code 3.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: compiler.wasm runs warp's commands under WASI (help, version, parse, repl) and emitter.warp repeats texts; bounced 2026-10-24 00:40: with it `rebuild pure` gives Error("not a text").

## Done
WASI command dispatch, text repeat, a local main merge in trees/wasm-commands-merge.

## Missing
Fix the rebuild-pure failure (and std_os_input refusal), compiler.warp conflict, re-Ready.

## How to test
Touches Rust: build its own warp/warp-host (thin LTO, one box slot, own CARGO_TARGET_DIR), then the host tests it names (tests/host/test_ffi.sh, test_host.sh …) plus the warp-only gate for its lib/ part; the Rust lane is paused, so this waits for it.

## Known failures
rebuild pure = Error("not a text") in the B2 batch.

## Next step
Fix the rebuild, merge main, re-Ready.
