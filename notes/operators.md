# wap's operator stage: design choices

The plan is notes/backbone.md "Operators"; the table is wap/preload/operators.wap, the mechanism
wap/core/parse/operators.wap, the printer wap/core/format.wap. What the code decided beyond the plan:

- **One parse, juxtaposition included.** The stage is an operator-precedence parse in which juxtaposition is two
  groups of the table: glued (`3ms`, `f(x)`, `a.b(c)`; group access, with `.`) and spaced (looser than every
  expression operator, tighter than `|>`, `->`, `=`). Juxtaposition builds a phrase, not a Call: stage 2 decides it.
  `print 1 + 2` is `[print, (Call + 1 2)]`. Asked the user whether `square 3 * 3` should be refused instead
  (wiki/operator.md and wiki/precedence.md disagree); the table's one relation changes either way.
- **Spacing picks the fixity** (the minus rule, for every operator): attached only to what follows → prefix, with
  juxtaposition before it (`a -b` is `[a, (Call - b)]`); attached only to what precedes → postfix where it has one,
  else infix (`1- -3`); both or neither → infix. At the start of a phrase or after an operator, prefix.
- **Unrelated on purpose:** shifts and arithmetic (`a << 1 + 2`), bit operators and comparisons (`a & b == c`).
  Comparisons, equality and ranges refuse chaining (`a < b < c`).
- **The English name is not an alias.** A row gives the symbol and its name (`+`, add); `add(a, b)` stays a call of
  add, and the binding stage knows add and `+` are one function from the row. An alias maps a spelling to a
  canonical word in a fixity (`plus` only infix), or anywhere (`≤`, `true`, `length` → count, `string` → text).
- **Glyph runs join in the stage:** the reader splits `=>` into `=` `>`; a glued run of symbols joins to the longest
  spelling the table knows. `x ⊛= y` of any infix ⊛ is a compound assignment without a row of its own.
- **Spelled out by format:** a later item of a space list led by a sign spacing decided prints `[a, -1]` in brackets
  and `print (-x)` in a phrase.
