# wap's rule engine: design choices

Code: wap/core/rules/patterns.wap (compile and match), templates.wap (build, hygiene), stages.wap (load, index,
termination, the stage-1 walk). Interface agreed with the lead 2026-11-17.

- **Rules are data.** kernel/forms.wap, core/sugar/*.wap and a program's own lines are read by wap's reader and
  operator stage and taken in when loaded: `pattern Name = shape` declares a pattern, `rule P => T` a rule. So a
  pattern is written in exactly the language it matches (operated code: `$a ÷ $b` is `(Call ÷ $a $b)`). The rule and
  pattern lines leave the program; what stays is surface.
- **Holes:** `$x`, `$x: Shape`, `$xs...` (the reader keeps a rest hole one word). A typed hole before more items,
  `(If $c: Node $then: Node …)`, reads as a colon key holding the rest of the phrase; normalization turns it back
  into a hole with a `type` fact and puts the items after it back in the list. v1 guards are shapes only (Node,
  Symbol, List, Key, Text, Number, Char, a declared pattern); a value type is refused until typing exists.
- **Constructors come from patterns.** In a template (and a pattern), `If($c, ø, $body)` names a declared pattern
  and builds its node `(If $c ø $body)`; any other capitalized call stays a call (a class constructor).
- **Indexed by head:** a phrase's first word, a form's name with its first word (`Call/+`), a key's operator; a
  pattern starting with a hole is tried everywhere. Among fitting rules the most specific (most literal parts) wins;
  two equally specific ones are refused naming both.
- **Walk:** top-down; a node is rewritten until no rule fits, then its parts. Built nodes take the rewritten code's
  place (stage.wap `staged`) and carry the fact RULE, the rule's written head.
- **Hygiene:** a name a template binds by itself (`t = $a`) is made fresh per use (`t·3`, fact `fresh`).
- **Termination at load:** the heads a template writes must not lead back to the rule's own head, unless each rule
  on that cycle writes less code than it matches and uses no hole twice.
- **Lowering** passes a declared form a rule built (`(If …)`) with its parts lowered.

## Open

- Hygiene has no run test yet: wap's typing can't yet print a variable holding a text.
- Lowering's hand cases (phrases, `if … then … else`, `=>`) move into core/sugar rules next.
