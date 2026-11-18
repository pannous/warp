# wap's rule engine: design choices

Code: wap/core/rules/patterns.wap (compile and match), templates.wap (build, hygiene), stages.wap (load, index,
termination, the stage-1 walk). Interface agreed with the lead 2026-11-17.

- **Rules are data.** kernel/forms.wap, core/sugar/*.wap and a program's own lines are read by wap's reader and
  operator stage and taken in when loaded: `pattern Name = shape` declares a pattern, `rule P => T` a rule. So a
  pattern is written in exactly the language it matches (operated code: `$a ÷ $b` is `(Call ÷ $a $b)`). The rule and
  pattern lines leave the program; what stays is surface.
- **Rest holes anywhere:** `$xs...` takes as few items as let the parts after it match (all that remain when last),
  so `if $c... then $a... else $b...` stops the condition at the first then. In a template `$xs...` splices the items,
  `$xs` writes them as one node (a phrase, the item itself, ø for none).
- **Repetition** (macros by example): `($p => $r)...` in a pattern takes items that each fit `$p => $r` and binds
  each hole to the sequence of its matches, under the rest key `$p...`; in a template `$p...` splices them and
  `Arm($p, $r)...` writes one Arm per match. A bracketed template list that grows takes its natural separator
  (block statements one a line, else commas).
- **Several statements:** `rule P => A; B` writes both (the `;` is looser than `=>`, so the rule takes the rest of
  its line); a statement sequence standing as a statement joins the enclosing statements, wherever it comes from.
  Braces keep meaning a Block. A repeated key `($field: $type)...` is a key of two holes, not a typed hole.
- **Heads of glued calls:** `f(x)` is indexed under `f Round`, then `glued Round`; a template's glued call of a kernel
  form, `Get($xs, $i)`, counts under the form's name, as `(Get …)` does.
- **Rule heads inside a phrase:** a phrase no rule fits whole, whose later word heads a rule fitting from there on,
  groups that part as the earlier words' last argument, as `print -x` does: `print if c then a else b`.
- **The rest of the If row:** `c ? a : b` (the operator table's choice: the colon closes `?`, below every operator,
  above a spaced call, so `print c ? a : b` is print(c ? a : b)) and
  `print "a" if c` (core/sugar/conditions.wap).
- **Refusals are rules too:** `rule P => refuse "fix-it"` refuses what P matches, by the usual specificity: the
  dangling else `if a then if b then x else y` (core/sugar/conditions.wap).
- **Holes:** `$x`, `$x: Type`, `$xs...` (the reader keeps a rest hole one word and gives a hole's `:` exactly one
  type, so `(If $c: Node $then: Node …)` is three typed holes). The type rides as the hole's `type` fact. A shape
  type (Node, Symbol, List, Key, Text, Number, Char, a declared pattern) is checked here; a value type (`$k: int`,
  `$xs: list<int>`) makes a stage-3 rule, kept in `typed_rules` and checked by typing (`fits_type`, the one
  seam to core/analyze/types.wap `fits_type`). Stage 1 runs `shape_rules` only.
- **Stages share one walker:** `rewritten_by(rules, code) as Fired` gives the code and whether any rule fired, so
  typing can run again after stage 3.
- **Constructors come from patterns.** In a template (and a pattern), `If($c, ø, $body)` names a declared pattern
  and builds its node `(If $c ø $body)`; any other capitalized call stays a call (a class constructor).
- **Indexed by head:** a phrase's first word, a form's name with its first word (`Call/+`), a key's operator; a
  list also under its bracket and separator, then its bracket alone, so a pattern starting with a hole, `$then... if $c`,
  waits under the shape it matches; only a bare hole is tried everywhere. Among fitting rules the most specific (most literal parts) wins;
  two equally specific ones are refused naming both.
- **Walk:** top-down; a node is rewritten until no rule fits, then its parts. Built nodes take the rewritten code's
  place (stage.wap `staged`) and carry the fact RULE, the rule's written head.
- **Modules:** a used module's rules apply to the program that uses it (`use` and `rule` build the language the
  program is written in): the `use` lines are read before shaping, each module is declared after the ones it uses.
- **Hygiene:** a name a template binds by itself (`t = $a`) is made fresh per use (`t·3`, fact `fresh`).
- **Termination at load:** the heads a template writes must not lead back to the rule's own head, unless each rule
  on that cycle writes less code than it matches and uses no hole twice.
- **Lowering** passes a declared form a rule built (`(If …)`) with its parts lowered.

## Open

- Hygiene has no run test yet: wap's typing can't yet print a variable holding a text.
- Lowering's hand cases (phrases, `=>`) move into core/sugar rules next; `if` already did.
