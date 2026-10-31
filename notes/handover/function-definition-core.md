# Parked: function-definition-core (def-first, 2026-10-31)

Every function definition in core/ and lib/ starts with `def` (3780 lines, scripts/defify.py --write core lib, which
also lists any left without). Smoke, self-check and gate tier 1 passed. Readers fixed for it: lib/rule_words.warp
first_parameter_type (core prelude_words without_defining_keyword), self-check.sh's extended-runtime scan,
case_tables.py and warp-crate (they write definitions).

Parked by the user's decision ("land the fixes alone", 2026-10-31): the rewrite gained nothing measurable on
compiling compiler.warp. The walk fixes found while measuring landed on main via phase-markers (notes/compile_speed.md
"Two walks"). Decision def-first's two collection passes (classes and types, then functions, after with_used) were
not built; user: discuss later. To revive: merge origin/main, rerun scripts/defify.py --write core lib, rebuild.
