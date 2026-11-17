# wap's operator stage: design choices

The plan is notes/backbone.md "Operators"; the table is wap/preload/operators.wap, the mechanism
wap/core/parse/operators.wap, the printer wap/core/format.wap. What the code decided beyond the plan:

- **One parse, juxtaposition included.** The stage is an operator-precedence parse in which juxtaposition is two
  groups of the table: glued (`3ms`, `f(x)`, `a.b(c)`; group access, with `.`) and spaced (looser than every
  expression operator, tighter than `|>`, `->`, `=`). Juxtaposition builds a phrase, not a Call: stage 2 decides it.
  `print 1 + 2` is `[print, (Call + 1 2)]`, `square 3 * 3` is square(9) (user, decision spaced-call-precedence;
  wiki/precedence.md's refusal line dropped).
- **Spacing picks the fixity** (the minus rule, for every operator): attached only to what follows → prefix, with
  juxtaposition before it (`a -b` is `[a, (Call - b)]`); attached only to what precedes → postfix where it has one,
  else infix (`1- -3`); both or neither → infix. At the start of a phrase or after an operator, prefix. Glued to a
  bracket after a blank it is called like any `f(x)`: `⊕(a, b)`.
- **An operator whose operand cannot follow is a name:** at the end of a phrase or before an infix-only operator
  (`fold +`, `prefix operator ⁻ := …`). No refusal for a missing operand is left.
- **A program's own operators** join the table before its phrases are read (a prescan of the whole program):
  `[infix|prefix|postfix|suffix] operator ⊕ …` in the group its word names (binds like `+`, prefix unary, postfix
  superscript; rows in the preload), a spelling the table has in that fixity is an overload, and
  `operator ⊕ has precedence above|below *` gives ⊕ a group of its own related to `*`'s. The declaration lines stay
  in the program, operated like any phrase, for stage 2 to define.
- **format drops brackets precedence makes needless** (`1 + (2 * 3)` → `1 + 2 * 3`): what a reading decided shows.
- **Unrelated on purpose:** shifts and arithmetic (`a << 1 + 2`), bit operators and comparisons (`a & b == c`).
  Comparisons, equality and ranges refuse chaining (`a < b < c`).
- **An operator's English name is a second name of the same function, joined by the row, never an alias** (lead,
  2026-11-17). A row gives the symbol and its name (`+`, add); neither is rewritten into the other: `add(a, b)` stays
  a call of add, `a + b` a call of `+`, and binding joins them from the row. As an alias, `add(a, b)` would read as
  prefix `+` of a tuple. An alias maps a spelling to a canonical word in a fixity (`plus` only infix), or anywhere
  (`≤`, `true`, `length` → count, `string` → text).
- **Glyph runs join in the stage:** the reader splits `=>` into `=` `>`; a glued run of symbols joins to the longest
  spelling the table knows. `x ⊛= y` of any infix ⊛ is a compound assignment without a row of its own.
- **Spelled out by format:** a later item of a space list led by a sign spacing decided prints `[a, -1]` in brackets
  and `print (-x)` in a phrase.

## Open

- **A program's `alias a => b` lines** (decision aliases-unified): a program alias would rewrite the line that declares
  it; decided with the rule engine.
- **Chained orderings** (`1 < x < 3` as `1 < x and x < 3`, warp's extensions/sugar/arithmetic.warp and its test
  test_equality_never_chains, no wiki page): the table refuses a chain of comparisons today. With the rule engine the
  comparison group can group from the left and a stage-1 rule rewrite the chain.
