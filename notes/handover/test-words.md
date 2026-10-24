# test-words: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3148 test words as lib/sugar/test_words.warp rules ([test-words](https://github.com/pannous/warp/issues/3148))
- Branch: `test-words` at `d9abe2d78`, 3 ahead / 288 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/rewrites.warp)
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: Test words `x is empty`, `x failed`, `x missing` as rules in lib/sugar/test_words.warp; readied express once, now a rewrites.warp conflict.

## Done
test_words.warp rules, merged with main once by the Integrator.

## Missing
Merge main (rewrites.warp), self-check, Ready.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Merge main, self-check, Ready; planner assigns.
