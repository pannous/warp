# optional-helpers: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: draft card hDLGm4 'compiler helpers that can answer ø declare list? instead of any (optional-helpers)' (Next)
- Branch: `optional-helpers` at `9376e3bb7`, 1 ahead / 1473 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/parser.warp)
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: Compiler helpers that can answer ø declare `list?` instead of `any` (decision optional-result, beautiful types); 5 lines, 1467 behind, parser.warp conflict.

## Done
The 5-line declaration change.

## Missing
Redo on today's main (the helpers moved), self-check.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Conflict in parser.warp only.

## Next step
Redo on main as a fresh small branch; planner assigns.
