# sugar-definitions: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3061 definitions' def/fun and var/let into… ([sugar-definitions](https://github.com/pannous/warp/issues/3061))
- Branch: `sugar-definitions` at `0f0cb389d`, 6 ahead / 387 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/parsing/parser.warp, core/passes/rewrites.warp, notes/rewrites.md)
- Kind: warp only
- Owner: Warp Code (grok-warp-code; parked 2026-10-23, handover notes/box/handover/warp-code.md) / grok-merger: that owner continues it when resumed (the planner reassigns it otherwise).
- State: Function keywords (def, fun, fn …) as early sugar rules in lib/sugar/functions.warp; readied once, now conflicts (parser.warp, rewrites.warp).

## Done
functions.warp rules.

## Missing
Land through sugar-declarations (which contains it).

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Land via sugar-declarations.
