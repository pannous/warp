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

## Update 2026-10-24 (Worker 1, card hDLe74)
- Merged origin/main in (merge, clean), then d0901b141: the own-type conversion diagnostic passes its statement as
  `about` (main's diagnose(fact, about)). `warp rebuild pure` and `self` pass, hellos too.
- Measured with the gate (`scripts/gate-probes.sh --base origin/main --folders`): tiers 0 and 1 green. Tier 2 (pure,
  whole tests/types) has 915 of 1833 failing, against 941 of 1821 on main c196fc916 run the same way: 39 cases newly
  pass (`t as number`, `'1/3' as number`, `as hex`, `as binary`, test_number_field, test_text_as_number), 13 newly fail.
- Known failures: every case of the branch's own tests/types/test_pure_conversions.warp fails (12 cases). 10 of them,
  every `Person as text` / `text(p: Person)` conversion, trap with "call stack exhausted"; :51 and :53 (`as list of T`)
  give "the operator . on int". tests/types/test_printable.warp:6 (`text(x)` of a class with `text(p: person)`) passed
  on main and now traps the same way. Next: find the recursion in a program's text conversion (probably
  `"P " + p.name` shown through the conversion itself), then make test_pure_conversions pass.
