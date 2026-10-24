# gap-element-array: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3200 x is a 100 element array declares x as 100 items ([gap-element-array](https://github.com/pannous/warp/issues/3200))
- Branch: `gap-element-array` at `58aa46837`, 6 ahead / 198 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: warp only
- Owner: Warp Code (grok-warp-code; parked 2026-10-23, handover notes/box/handover/warp-code.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: `x is a 100 element array` as a rule in lib/sugar/lists.warp plus a test, but the parser change that made the rule fire was reverted (it made pure builds loop), so the rule doesn't fire yet.

## Done
The rule, probes/sugar/element_array, tests/lists/test_element_arrays.warp (0/2).

## Missing
A rule shape the parser reads, or a parser case with Supervisor's OK.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
tests/lists/test_element_arrays.warp 0/2 (the rule never fires).

## Next step
Find a rule shape that fires; don't merge before (it adds 2 failing cases).
