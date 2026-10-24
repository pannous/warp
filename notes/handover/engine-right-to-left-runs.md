# engine-right-to-left-runs: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3154 engine: right-to-left runs ([engine-right-to-left-runs](https://github.com/pannous/warp/issues/3154))
- Branch: `engine-right-to-left-runs` at `1873b3df7`, 6 ahead / 220 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: warp only
- Owner: Warp Code (grok-warp-code; parked 2026-10-23, handover notes/box/handover/warp-code.md); parked by the user (card in Later): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Rewrite engine F13: runs of statements rewritten right to left so clause-style definitions (`fib(0) := 0; fib(n) := …`) work; merges cleanly but the user parked the feature.

## Done
Engine change plus tests, merged with main.

## Missing
The user's decision to unpark; then self-check and Ready.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Not measured since the last main merge.

## Next step
Stays parked until the user unparks clause-style definitions.
