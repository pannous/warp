# pure-match-guard-arms: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: draft card hDLGo4 'pure lane: guard and bare-name binding arms in node matches (pure-match-guard-arms)' (Next)
- Branch: `pure-match-guard-arms` at `658c010a6`, 1 ahead / 227 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: warp only
- Owner: Warp Code 2 (grok-warp-code-2; parked 2026-10-23, handover notes/box/handover/warp-code-2.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Match step A: core/node_matches.warp +36/-4, guard arms `p if c` and bare-name binding arms in the pure lane; untested.

## Done
node_matches.warp change; probes in /workspace/probes-mg/.

## Missing
A rebuild, a result test, self-check; known gap: no scope info, so an already-bound name in an arm binds instead of comparing; `guarded_arm` uses a `test == yes` marker.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Never run.

## Next step
Rebuild + probes, add a result test, self-check, Ready (match plan A, approved).
