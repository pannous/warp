# The built-in node's cases (card node-enum, decision compiler-node-enum)

lib/compiler/node.warp is the one warp declaration of the built-in node: `enum Node` case for case as src/node/mod.rs,
and the enums Op, Bracket, Separator. It is the definition, not a copy: no class is made of Node, its cases are the
kinds of the $Node the runtime already has (src/lowering/types/node_cases.rs).

- `Node` where a type is written (`f(n: Node): Node`) is any node. `Node.Key`/`Node::Key` is the case, `Node.True` yes,
  `Node.Empty` ø.
- Patterns (class_methods/destructuring.rs `built_in_case`): `Key(left, op, right)`, `List(items, bracket, separator)`,
  `Symbol(name)`, `Text(t)`, `Number(x)`, `Char(c)` test the kind (`is key`; a list is `bracket_of(n) != ø`, since ø is
  a list) and read the parts with key_of/op_of/value_of, name_of, bracket_of, separator_of. Nested patterns work.
  Error, Data, Meta and Type have no reader yet.
- Constructions after class_methods: `Key(a, op, b)` is `key(a, Op·text(op), b)`, `List(items, b, s)` is
  `separated(bracketed(items, Bracket·text(b)), Separator·text(s))`. Symbol/Text/Number/Char(x) were conversions already.
- Op, Bracket, Separator are plain enums (`print Op.Add` → Add), put in front of a program that names them or Key/List,
  with `Op·text` and `Op·case` (match functions; a text passes through both, so `key(a, "+", b)` keeps working).
  `Op.Add as text` is "+", `"+" as Op` is Add (plain_enums.rs `is_spelled`). A pattern's op/bracket/separator field is
  the case; op_of itself still gives the text until node-rewrite flips it (Interviewer's decision).
- A program's own type or function of the same name wins (`class Key`, `Key(a, b) := …`, `enum Op {…}`).
- Twins: tests/node/test_node_cases.rs holds node.warp to the Rust Node (an exhaustive match), OP_CODES and SEPARATORS.
  $Node of type_manager.rs is not compared yet (emitter.warp derives it later).

Known gaps: Sub and Neg are both spelled "-", so `"-" as Op` and the op field of a negation read Sub. `Bracket.None as
text` stays the name: None is a case of all three enums, so its owner is unknown after `Bracket.None` became None.

Also fixed on the way: an enum case in any class pattern compares (`P(Color.Red, n)` no longer binds a variable Red).
