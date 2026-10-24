# libgap-as-conversions: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3171 pure lane: x as T conversions, incl. units ([pure-as-conversions](https://github.com/pannous/warp/issues/3171))
- Branch: `libgap-as-conversions` at `aefe652be`, 10 ahead / 264 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: warp only
- Owner: Warp Lib Gap (grok-warp-lib-gap; parked 2026-10-23, handover notes/box/handover/Warp Lib Gap.md): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Pure-lane conversions: a program's `A as B := body`, `text(p: P)`, `t as number`, `v as list of T`; warp only, merges textually clean but the merged compiler fails `rebuild pure` (main changed diagnose's parameters).

## Done
definitions.warp conversions, emitter/of_lookups bits, lists.warp rule, tests/types/test_pure_conversions.warp.

## Missing
Adapt to main's diagnose(fact, about) (definitions.warp:314 calls diagnose with one argument), then self-check, test run, Ready (card pure-as-conversions); needed by the Quantity units plan.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Merged onto main d665c3937 (Integrator, 2026-10-24 10:01 CEST): `warp rebuild pure` fails: diagnose needs a value for parameter about (it has no default) at lib/compiler/definitions.warp:314 (main's diagnose(fact, about) since 88122b18f). Merges textually clean, so it looks ready but isn't.

## Next step
Pass the statement as `about` in the conversion diagnostics, rebuild pure+self, run tests/types, Ready.
