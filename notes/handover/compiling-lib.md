# compiling-lib: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #2701 compiling core warns about its own lines ([compiling-lib](https://github.com/pannous/warp/issues/2701))
- Branch: `compiling-lib` at `2a3d838b8`, 2 ahead / 747 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/parsing/parser.warp)
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: core compiles without its own local-or-global warnings and indexing hints (8 lines); 741 commits behind, parser.warp conflict.

## Done
used_module_sources' locals renamed, word#1 for word[0].

## Missing
Redo the 8 lines on today's main, self-check.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Conflict in parser.warp only.

## Next step
Redo on main as a fresh branch (small); planner assigns.
