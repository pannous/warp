# pure-refusal-go: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3150 pure lane: go and await refuse with unknown name ([pure-refusal-go](https://github.com/pannous/warp/issues/3150))
- Branch: `pure-refusal-go` at `553cfc43e`, 2 ahead / 303 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/rewrites.warp)
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: go/await as lib/sugar/tasks.warp rules (task run at once, await within, stop, cancel): 57 of 128 blocked tests pass; WIP, rewrites.warp conflict; on hold until pure class operators work.

## Done
tasks.warp rules.

## Missing
rewrites.warp conflict, go in call arguments and loops, `await … within`.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
71 of 128 go/await tests still fail.

## Next step
On hold (on/go wait for classes and operators in pure); planner assigns later.
