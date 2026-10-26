# sugar-declarations: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3120 sugar: var/let declaration words into lib/sugar ([sugar-declarations](https://github.com/pannous/warp/issues/3120))
- Branch: `sugar-declarations` at `53fd74273`, 9 ahead / 387 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/parser.warp, core/rewrites.warp, lib/sugar/declarations.warp, notes/rewrites.md)
- Kind: warp only
- Owner: Warp Code (grok-warp-code; parked 2026-10-23, handover notes/box/handover/warp-code.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: var/let/const/final/val as early sugar rules in lib/sugar/declarations.warp (contains sugar-definitions); conflicts in parser.warp, rewrites.warp, declarations.warp (sugar-foreign-forms added the same file on main).

## Done
Rules, suite 626/0 when readied.

## Missing
Merge main onto first_sugar and main's declarations.warp, self-check.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Merge main, self-check, Ready (lands sugar-definitions too).

## Progress (box worker w1, 2026-10-24 18:20 CEST)
- 2af83a522 (pushed): early sugar = sugar/functions + sugar/declarations, also applied to each library definition a program reaches (prelude_words with_reached); `{$b...}` of a one-statement block (`{sum xs}`) binds the block whole. `rebuild pure` + pure hello OK, `rebuild self` + self hello OK, probes/sugar/{definitions,declarations} same in both lanes.
- Quick run of the 10 files that failed at e851ec181: branch 147 / main 151 of 535, 0 new failures, 4 fixed.
- Full comparison running: /workspace/cow/w1-cmp.sh output /workspace/cow/w1-sugar.out, logs /workspace/cow/w1-cmp-w1-sugar-w1-tests-sugar{,--self}.log; compare with `python3 /workspace/cow/w1-diff.py <branch log> /workspace/cow/w1-cmp-w1-main-w1-tests-sugar.log`.
- The WIP patches /workspace/cow/w1-sugar-wip*.patch are superseded by 2af83a522.
- Full pure run at 2af83a522: 968/2351 vs main 906; of the 66 "new", 59 were `timed out after 30 s` in tests/wasm/test_wasm (box load ~33), 7 real: class-body methods `def f(): x + 1` (types/test_class_methods 22-25, types/test_dispatch 14,18,20): a class body is a type view the early sugar never enters, and definitions.warp no longer strips the keyword. Fixed in class_methods.warp member_read (early_sugared per member): those two files now 7/24 = main 7/24.
- At b98454ece: the 10 files failing at e851ec181 rerun at low load: branch 147 / main 151 of 535, 0 new, 4 fixed (the 59 test_wasm timeouts were load). Next: rebuild self + self hello at b98454ece, then todo ready.
- 19:10: 4d1218d6e confirmed on origin (git ls-remote). Self check queued: /workspace/cow/w1-selfonly.sh (pid in w1-selfonly.pid, out w1-selfonly.out) runs after the old w1-cmp self-test job ends (timeout ~19:19).
- 19:30: rebuild self + self hello green at 6b86fd606; readied.
