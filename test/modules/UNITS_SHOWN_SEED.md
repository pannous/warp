# units_shown seed warning (card lib-units-shown-warning)

Stdout may show:

`warning: the rule word units_shown is not in this compiler (an older seed): its rules are skipped`

when `WARP_LIB` points at a tree whose `lib/units.warp` / `lib/rule_words.warp` reference `units_shown`, but the warp binary’s baked seed predates that rule word (e.g. native built 2026-11-03). The warn is printed from **core**/passes/rewrites.warp (`skipped_rule_word`); the program rewrite in `lib/units.warp` is intentional (unit-typed fields show in their unit).

**Not a lib false positive.** Do not drop `units_shown` or change money amounts to silence it. Fix: rebuild pure / refresh the seed so the compiler includes `units_shown`. Pairs tests do not assert stderr. See also `test_money_split_amounts.pairs.warp` and `/workspace/handover/lib-units-shown-warning.md`.
