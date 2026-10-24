# host-from-warp: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3094 can the host itself be built ahead of time from… ([host-from-warp](https://github.com/pannous/warp/issues/3094))
- Branch: `host-from-warp` at `a3c6f5641`, 4 ahead / 393 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: notes/host_from_warp_eval.md)
- Kind: warp only
- Owner: grok-merger / warp-bootstrap (parked 2026-10-23, handover notes/box/handover/warp-bootstrap.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Evaluation of a warp host written in warp: a driver passing test_host.sh 86/86 in-process and spawned; conflicts in notes/host_from_warp_eval.md. The user's branch `me` has the same tip.

## Done
The driver, the evaluation notes.

## Missing
Resolve the notes conflict (most host commits also ride in warp-host-wasm-load, now on main), the C-import follow-up card once pure-c-imports merges.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Unknown after a main merge.

## Next step
Merge main (notes only), rerun test_host.sh, Ready.
