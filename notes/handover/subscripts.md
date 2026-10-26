# subscripts: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: draft card hDLGqg 'xs[i] and slices as lib/sugar/subscripts.warp rules on first_sugar (subscripts)' (Next)
- Branch: `subscripts` at `b980d4c91`, 4 ahead / 336 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/passes.warp, core/rewrites.warp)
- Kind: warp only
- Owner: warp-layers (Mac session) continues it.
- State: `xs[i]` and slices as rules in lib/sugar/subscripts.warp, run in a new written_sugar step; main has its own first_sugar phase now, conflicts in passes.warp and rewrites.warp. Its old card retire-hybrid-lane is Done.

## Done
subscripts.warp rules, the `where $b is ø` fix in rewrites.

## Missing
Port the rules onto first_sugar (drop written_sugar), resolve conflicts, self-check.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Port onto first_sugar, self-check, Ready.
