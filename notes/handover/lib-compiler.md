# lib-compiler: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #2760 lib/compiler/*.warp still test emptiness with ==… ([lib-compiler](https://github.com/pannous/warp/issues/2760))
- Branch: `lib-compiler` at `b17879384`, 17 ahead / 676 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/analyzer.warp, lib/compiler/builtin_signatures.warp, lib/compiler/emitter.warp, lib/compiler/foreign_forms.warp, lib/compiler/kind_changes.warp …)
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: lib/compiler tests emptiness with `is empty` / plain `if x` (rule empty-is-not-missing); the rewrite broke the self build (bisect notes on the card), 670 behind.

## Done
The rewrite (one commit landed as fa09d9c2b).

## Missing
Redo by halves on main, self-check each half.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Self build broken by the full rewrite.

## Next step
Redo in halves on main; planner assigns.
