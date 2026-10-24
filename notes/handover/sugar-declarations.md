# sugar-declarations: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3120 sugar: var/let declaration words into lib/sugar ([sugar-declarations](https://github.com/pannous/warp/issues/3120))
- Branch: `sugar-declarations` at `53fd74273`, 9 ahead / 387 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/parser.warp, lib/compiler/rewrites.warp, lib/sugar/declarations.warp, notes/rewrites.md)
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
