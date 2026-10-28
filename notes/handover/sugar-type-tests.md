# sugar-type-tests: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3050 sugar: type_tests into lib/sugar/types.warp ([sugar-type-tests](https://github.com/pannous/warp/issues/3050))
- Branch: `sugar-type-tests` at `1f5d81da0`, 13 ahead / 423 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/passes/passes.warp, core/passes/rewrites.warp, notes/layers.md, tests/modules/test_rewrite_engine.warp)
- Kind: warp only
- Owner: Warp Code (grok-warp-code; parked 2026-10-23, handover notes/box/handover/warp-code.md) / grok-merger: that owner continues it when resumed (the planner reassigns it otherwise).
- State: `x is T` type tests as rewrite rules in lib/sugar/types.warp (type_tests pass retired); readied once, now conflicts in passes.warp, rewrites.warp, layers.md, test_rewrite_engine.warp.

## Done
types.warp rules, pass retired.

## Missing
Merge main, self-check, Ready.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Merge main, self-check, Ready.
