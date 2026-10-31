# Unit spellings and unit rules (cards units-unknown-name, work-down, 2026-10-31)

- lib/unit_names.warp: the unit table, the SI prefixes (on m g s K Hz N L), the long names with long prefixes, one
  plural rule (-s/-es), °C/°F with their zero. lib/units.warp reads it: `quantity(3, "kilograms")` is 3kg,
  `quantity(212, "°F") == quantity(100, "°C")`, two temperature scales don't add.
- lib/rule_words.warp unit_bound: a unit after an expression multiplies its last amount (`x > 1500 m`).
- Literal derived spellings (`1 Gs`, `3 kilometers`) wait for card module-rule-unnamed: a module rule comes only when
  the program names its head word or a word of its `$u in [...]` list (core/passes/rewrites.warp own_rules_applied).
  Parked on branch headless-module-rules. A derived word alone also loads no module (card word-index).

Gotchas met:
- A new rule word must also join the `rule_words` map at the end of lib/rule_words.warp, otherwise "not in this
  compiler (an older seed)". Each rule_words.warp change is a compiler build (~450 s CPU).
- `$symbol` is a hole kind, not a free hole name: name holes `$written` etc.
- The pairs harness counts a truthy list as yes: `1 km == 1000 m` parses as a list and "passes" (card
  spaced-unit-comparison). Compare runs with the full error text, not just pass/fail.
- A top-level value of a module used by a module can be unknown when another module's words reach it (card
  unknown-name): write such constants as functions.
- `warp format yes` breaks .pairs.warp tabs (card format-tab-spacing).
- lib/units.warp is bundled into the compiler, and its rules also apply when the compiler compiles itself: a rule
  firing there pulls units in (Quantity's shown() beside diagnostics.warp's shown: "pattern·clause·shown"). Every
  units rule change needs scripts/self-check.sh, not only smoke. A `$u in [s, …]` listing also takes the char 's'
  (card char-literal-matching), so unit rules guard with `$u is symbol`.
- `rewrite (… :=` on one line reads as a definition of the word rewrite (prelude_words.warp name_defined_in), so the
  word index sent every rewrite to that module; fixed in core: a line led by `rewrite` defines no word (card
  units-rewrite-assign).
- Finding where a module rule fires in the compiler: give its template a marker call
  (`=> unit_rule_fired_here($bound)`), run `warp lower compiler.warp` from core/ with WARP_LIB=<worktree>/lib (3 min,
  no rebuild), grep the marker (card module-rule-trace for a real trace).
