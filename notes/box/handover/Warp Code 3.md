# Handover: Warp Code 3 (claim grok-warp-code-3), 2026-10-23 ~14:40 (Europe/Berlin)

Parked at the user's request. No new work accepted. Rules picked up today (all in common.md): only the pure pipeline counts;
tests are result-based only; no new pass, pass-list entry or lib/compiler/*.warp file without the user's yes; lib/compiler edits
over ~40 lines or new lowering/special cases need Supervisor's OK; no underscore compound folder names (nest them: runtime/extended);
core = compiler + runtime + extensions, lib = ordinary Warp modules; domain features (units, time) are class libraries in lib/.

## Ready / queued (merger owns them)
| card | branch | tip | notes |
|---|---|---|---|
| local-shadows-lib-function | local-shadows-lib-function | 0d26380eb | express. Rust fix src/pipeline.rs +3 (shadowing_bindings rerun after modules::resolve). pure/self were already right. Red main as reported wasn't reproducible. |
| pure-error-positions | pure-error-positions | d07ade09d | 6/6 probes/pure_positions match Rust; 0 test flips; gaps noted on the card |
| sugar-foreign-forms | sugar-foreign-forms | 8bd26ddc3 | queued. 21 rules. 6 probes expect values pure can't reach yet (by design). The self class-field failures (`bytes_between`) come from main; the merger bisects. |

## runtime-split stack: pushed + Ready 15:00 (runtime-split 2f641511b, pure-big-int-power dbfc755c0, pure-big-int-ops a3aebff4c, time-arithmetic-library d0d603745)
Merge order: runtime-split -> pure-big-int-power -> pure-big-int-ops -> time-arithmetic-library. All four held by the merger until re-readied.
- Done today: runtime_extended/ renamed to lib/compiler/runtime/extended/ (+ runtime/extended.warp root); durations.warp is a real file in runtime/
  (only the extended root loads it); `warp <folder>` says it's a folder (src/main.rs), so each_runtime_part_runs_alone passes; main merged
  (130713414) -> runtime-split 2f641511b; cascaded: tal 931250a23, power dbfc755c0, ops a3aebff4c.
- time-arithmetic-library: time_arithmetic.warp moved to lib/time_arithmetic.warp (d0d603745); the compiler must NEVER load it
  (Span.plus would route every compiler + through operator dispatch). Plain symlink lib/compiler/time_arithmetic.warp is kept for the frozen
  tests/parser/test_self_time_arithmetic.rs.
- The self lane needs runtime-split's own binary (sol-warp-latest lacks the extended root -> `extended_zeros` refusal). Rebuild the box binary after it lands.
- Trees: /workspace/cow/trees/{runtime-split,time-arithmetic-library,pure-big-int-power-2 (branch pure-big-int-power),pure-big-int-ops}.
  Progress/logs: /workspace/cow/scratch/rx-progress.md, /workspace/cow/scratch/rx/.

## Waiting on runtime-split landing
- wasm-commands: tree /workspace/cow/trees/wasm-commands, local merge of main 54756a917 (NOT pushed; origin has 66308daf8). Verified rebuild pure +
  hello OK (scratch/wc/rebuild.log). It touches runtime/wasi.warp, which runtime-split moved: after runtime-split lands, re-merge main, push,
  todo ready. wasm-commands-merge (214c5a28f) is then obsolete: drop it. The user approved pushing both branches.

## Parked / open cards (mine, not started)
- pure-time-arithmetic-wiring (Later): waits for pure-class-operators (Warp Classes 3b, then Warp Signals). The emitter plan was rejected; the path is
  lib/sugar literal rules (time/duration literals -> constructor calls) + `rewrite now => instant_at(clock())`, compiled class library.
  Record: Mac scratch/pure-time-arithmetic-wiring-plan.md (box /workspace/cow/scratch/ptaw-plan.md). Supervisor pings when class operators land.
- pure-floor-div-int-op (Next): `print(-1÷6)` crashes 'not an Int operator: FloorDiv' in src/wasm_emitter/numbers/big_int.rs:275 (test_self_floor_division);
  a Rust fix is allowed. Scratch /workspace/cow/scratch/pfd/. Release or hand to someone.
- pure-big-int-overflow (Soon), type-zoned (after runtime-split).
- Recheck comprehensions_run_pure once sugar-comprehensions has landed (it's on main now: rewrites.warp has comprehension_sugar_module_names).

## Cleanup after merges
`todo done` per card (needs the linked commit); `boxtree rm <card>` from /workspace/cow/trees/grok-warp-code-3 for: runtime-split,
time-arithmetic-library, pure-big-int-power-2, pure-big-int-power, pure-big-int-ops, local-shadows-lib-function, sugar-foreign-forms,
wasm-commands, wasm-commands-merge; delete merged branches. The stray tree /workspace/cow/trees/pure-floor-div-int-op is already gone.

## Box notes
- box-slot.sh can return 0 for an OOM-killed job: check the log/dmesg, not the exit code. Two cold `rebuild self` runs at once OOM.
- Pushing from the box: `git push origin <branch>`. Then on the Mac: `git fetch origin <b> && git branch -f <b> origin/<b>` before `todo ready`
  (todo compares the local branch).

## Status
2026-10-23 15:00 (Europe/Berlin): all four stack branches pushed and Ready; Supervisor 2 told. Executors done. Lone Rust-lane quirk: running lib/time_arithmetic.warp standalone errors at `code + 1` (Span.plus dispatch), as before the move; the 12 time repros pass. No jobs running. Parked.
