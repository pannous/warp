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
