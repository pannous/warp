# compiler-library-source: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3203 prelude, math, list and regex read from lib/ ([compiler-library-source](https://github.com/pannous/warp/issues/3203))
- Branch: `compiler-library-source` at `cdaa8fe25`, 2 ahead / 270 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: core/parser.warp)
- Kind: warp only
- Owner: Warp Wrapper (grok-warp-wrapper; session parked 2026-10-23, handover notes/box/handover/warp-wrapper.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: prelude, math, list and regex are read from the run's lib/ (library_source), so editing them needs no compiler rebuild; conflicts with main in core/parser.warp.

## Done
library_source and a host test (an alias added to WARP_LIB's prelude.warp works with an older compiler).

## Missing
Resolve the parser.warp conflict (main changed run_library's neighbourhood), self-check, Ready again.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
None known on the branch itself; the merge conflict is the blocker.

## Next step
Merge main, resolve parser.warp, self-check, `todo ready`.
